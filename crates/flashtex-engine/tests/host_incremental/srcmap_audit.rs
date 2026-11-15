//! Lane AUDIT-SRCMAP: the source map (`SOURCES`, `SPAN_SHIFT`, each
//! GLYPH's `col`) and `diag-v1` across incremental edits, through the
//! socket host as the app uses it (accepting `diag-v1` and
//! `span-shift-v1`). After every edit and every compile that settles the
//! `.aux` after it, every page the client holds and every `DIAG` of the
//! compile must equal a fresh host's compile of the same files: spans
//! resolved to (file relative to the project, line), columns, the
//! diagnostics' places, ranges, macro chains and definition sites.
//!
//! The documents: a multi-file project (`\input` files, one read twice, an
//! `\include`d chapter, a file read inside a box) with errors inside
//! macros defined in another file; and calls BOX-MEMO records in an edit's
//! window and replays at the next keystroke (a measure call that defines a
//! macro, a draw call whose glyphs carry spans).
//!
//! Skips where there is no TeX Live (e.g. CI).

use super::*;
use flashtex_display_list::diag::Diag;

const ACCEPT: &[&str] = &[
    flashtex_display_list::diag::CAPABILITY,
    flashtex_display_list::SPAN_SHIFT_CAPABILITY,
];

/// The edits of one compile, in order: in each, `find` (which must occur
/// once in the file as the edits before it left it) becomes `replace`.
struct Step {
    edits: Vec<(&'static str, String, String)>,
    what: String,
}

fn step(file: &'static str, find: &str, replace: &str, what: &str) -> Step {
    steps(&[(file, find, replace)], what)
}

fn steps(edits: &[(&'static str, &str, &str)], what: &str) -> Step {
    Step {
        edits: edits
            .iter()
            .map(|&(f, a, b)| (f, a.to_string(), b.to_string()))
            .collect(),
        what: what.into(),
    }
}

/// `p` relative to the project (`""`) or the output directory (`<out>/`).
fn rel(p: &str, root: &Path, out: &Path) -> String {
    for (d, tag) in [(root, ""), (out, "<out>/")] {
        let canon = std::fs::canonicalize(d).unwrap_or_else(|_| d.to_path_buf());
        for pre in [format!("{}/", d.display()), format!("{}/", canon.display())] {
            if let Some(rest) = p.strip_prefix(pre.as_str()) {
                return format!("{tag}{rest}");
            }
        }
    }
    p.to_string()
}

/// The compile's `DIAG`s as text, without what differs between two hosts
/// and two directories: the compile id, span ids (each resolved to its
/// file and line as the client holds it) and the directories in paths.
/// A `DIAG`'s span that does not name its own (file, line) is reported.
fn norm_diags(view: &View, diags: &[Diag], root: &Path, out: &Path) -> Vec<String> {
    let dirs: Vec<(String, &str)> = [(root, "<ROOT>"), (out, "<OUT>")]
        .iter()
        .flat_map(|(d, tag)| {
            let canon = std::fs::canonicalize(d).unwrap_or_else(|_| d.to_path_buf());
            [
                (canon.display().to_string(), *tag),
                (d.display().to_string(), *tag),
            ]
        })
        .collect();
    let span = |s: Option<i64>| -> String {
        match s {
            None => "-".into(),
            Some(s) => match view.spans.get(&(s as u32)) {
                Some(&(f, l)) => format!("{}:{l}", rel(&view.files[&f], root, out)),
                None => format!("undeclared span {s}"),
            },
        }
    };
    diags
        .iter()
        .map(|d| {
            let mut d = d.clone();
            d.id = 0;
            let mut spans = vec![span(d.span)];
            if let (Some(_), Some(f), Some(l)) = (d.span, &d.file, d.line) {
                let want = format!("{}:{l}", rel(f, root, out));
                if spans[0] != want {
                    spans.push(format!("SPAN NAMES {} BUT THE DIAG IS AT {want}", spans[0]));
                }
            }
            d.span = None;
            if let Some(e) = d.end.as_mut() {
                spans.push(span(e.span));
                e.span = None;
            }
            for f in d.trace.iter_mut() {
                if let Some(l) = f.loc.as_mut() {
                    spans.push(span(l.span));
                    l.span = None;
                }
                if let Some(l) = f.def.as_mut() {
                    spans.push(span(l.span));
                    l.span = None;
                }
            }
            let mut s = format!("{d:?} spans {spans:?}");
            for (dir, tag) in &dirs {
                s = s.replace(dir.as_str(), tag);
            }
            s
        })
        .collect()
}

/// Compile the copy `p2` (output `o2`) from scratch in the host at `sock`
/// and compare every page and every `DIAG` with `view`'s and `diags`.
#[allow(clippy::too_many_arguments)]
fn compare_all(
    sock: &Path,
    view: &View,
    diags: &[Diag],
    (proj, out): (&Path, &Path),
    (p2, o2): (&Path, &Path),
    main: &str,
    what: &str,
    bad: &mut Vec<String>,
) {
    let mut s = Client::connect_accepting(sock, ACCEPT).unwrap();
    let mut sv = View::default();
    let mut rs = CompileRequest::new(1, p2.to_str().unwrap(), main);
    rs.output_dir = Some(o2.to_str().unwrap().into());
    let so = compile(&mut s, &mut sv, &rs);
    let _ = s.bye();
    assert_eq!(
        so.done.str_field("mode"),
        Some("cold"),
        "{what}: {}",
        so.done
    );
    if sv.count != view.count {
        bad.push(format!(
            "{what}: {} pages, scratch {}",
            view.count, sv.count
        ));
        return;
    }
    assert_eq!(view.pages.len(), view.count, "{what}: pages missing");
    for i in 0..view.count as u32 {
        let a = resolve(view, i, proj, out);
        let b = resolve(&sv, i, p2, o2);
        if a != b {
            bad.push(format!(
                "{what}: page {i} differs from the scratch compile: {}",
                describe_difference(&a, &b)
            ));
        }
    }
    let (a, b) = (
        norm_diags(view, diags, proj, out),
        norm_diags(&sv, &so.diags, p2, o2),
    );
    assert!(!b.is_empty(), "{what}: the document has diagnostics");
    if std::env::var_os("AUDIT_SRCMAP_SHOW").is_some() {
        eprintln!("{what}: scratch DIAGs:\n  {}", b.join("\n  "));
    }
    if a != b {
        let k = (0..a.len().max(b.len()))
            .find(|&k| a.get(k) != b.get(k))
            .unwrap();
        bad.push(format!(
            "{what}: DIAG #{k} of {} (scratch {}) differs:\n  edit:    {}\n  scratch: {}",
            a.len(),
            b.len(),
            a.get(k).map_or("(none)", |s| s.as_str()),
            b.get(k).map_or("(none)", |s| s.as_str())
        ));
    }
}

/// The first page with a glyph from line `line` of `file`.
fn page_of(view: &View, file: &str, line: u32) -> Option<u32> {
    let file = format!("/{file}");
    view.pages
        .iter()
        .find(|(_, p)| {
            p.items.iter().any(|it| {
                matches!(it, Item::Span(s) if *s != 0
                    && view.spans.get(s).is_some_and(|&(f, l)| l == line
                        && view.files[&f].ends_with(&file)))
            })
        })
        .map(|(i, _)| *i)
}

/// Open the project `files` (main file `main`) in a resident host started
/// with `env`, make each step as an edit (the edited page as the
/// viewport), and compare each compile, and each compile that settles the
/// `.aux` after it, with a fresh host's. Returns the differences.
fn audit(
    name: &str,
    files: &[(&str, String)],
    main: &str,
    steps: &[Step],
    env: &[(&str, &str)],
) -> Vec<String> {
    audit_counting(name, files, main, steps, env).0
}

/// [`audit`], and the BOX-MEMO hits of the edited session's host (the
/// `boxmemo` counts of its last edit's `DONE`).
fn audit_counting(
    name: &str,
    files: &[(&str, String)],
    main: &str,
    steps: &[Step],
    env: &[(&str, &str)],
) -> (Vec<String>, i64) {
    let mut hits = 0;
    let base = common::fresh_dir(&format!("flashtex-srcmap-{name}"));
    let _ = std::fs::remove_dir_all(&base);
    let (proj, out) = (base.join("proj"), base.join("out"));
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    for (f, t) in files {
        std::fs::write(proj.join(f), t).unwrap();
    }
    let host = start_host_env(&format!("{name}-a"), env);
    let scratch = start_host_env(&format!("{name}-b"), env);
    let mut c = Client::connect_accepting(&host.1, ACCEPT).unwrap();
    let mut view = View::default();
    let mut bad = vec![];
    let mut id = 0;
    let settle =
        |c: &mut Client, view: &mut View, bad: &mut Vec<String>, id: &mut i64, what: &str| {
            for n in 0..4 {
                let (p2, o2) = snapshot(&base, &proj, &out, &format!("{id}s{n}"));
                *id += 1;
                let o = compile(c, view, &req(*id, &proj, &out, main));
                if o.done.str_field("mode") == Some("unchanged") {
                    break;
                }
                eprintln!("{name}: {what}, compile {n}: {}", o.done);
                compare_all(
                    &scratch.1,
                    view,
                    &o.diags,
                    (&proj, &out),
                    (&p2, &o2),
                    main,
                    &format!("{what} (compile {n})"),
                    bad,
                );
            }
        };
    settle(&mut c, &mut view, &mut bad, &mut id, "open");
    for (k, s) in steps.iter().enumerate() {
        let mut texts: BTreeMap<&str, String> = BTreeMap::new();
        let mut edits = vec![];
        let mut viewport = None;
        for (file, find, replace) in &s.edits {
            let text = texts
                .entry(file)
                .or_insert_with(|| std::fs::read_to_string(proj.join(file)).unwrap());
            assert_eq!(
                text.matches(find.as_str()).count(),
                1,
                "{name} step {k} ({}): {find:?} not once in {file}",
                s.what
            );
            let at = text.find(find.as_str()).unwrap();
            let line = text[..at].matches('\n').count() as u32 + 1;
            viewport = viewport.or_else(|| page_of(&view, file, line));
            edits.push(Edit {
                path: (*file).into(),
                offset: at as u64,
                delete: find.len() as u64,
                insert: replace.clone(),
            });
            text.replace_range(at..at + find.len(), replace);
        }
        let (p2, o2) = snapshot(&base, &proj, &out, &format!("{k}"));
        for (file, text) in &texts {
            std::fs::write(p2.join(file), text).unwrap();
        }
        id += 1;
        let mut r = req(id, &proj, &out, main);
        r.edits = edits;
        r.viewport = viewport;
        let o = compile(&mut c, &mut view, &r);
        let files: Vec<&str> = s.edits.iter().map(|e| e.0).collect();
        let what = format!("step {k} ({}: {})", files.join(", "), s.what);
        eprintln!("{name}: {what}: {}", o.done);
        if let Some(n) = o
            .done
            .get("stages")
            .and_then(|s| s.get("boxmemo"))
            .and_then(Json::as_array)
            .and_then(|a| a.get(1))
            .and_then(Json::as_i64)
        {
            hits = n;
        }
        compare_all(
            &scratch.1,
            &view,
            &o.diags,
            (&proj, &out),
            (&p2, &o2),
            main,
            &what,
            &mut bad,
        );
        settle(&mut c, &mut view, &mut bad, &mut id, &what);
    }
    let _ = c.bye();
    drop(host);
    drop(scratch);
    if bad.is_empty() {
        let _ = std::fs::remove_dir_all(&base);
    }
    (bad, hits)
}

fn para(tag: &str, i: usize) -> String {
    format!(
        "{tag} paragraph {i} has enough plain words in it to fill most of two lines \
         of the page, so that a few of them together make a page of the document.\n\n"
    )
}

fn paras(tag: &str, n: std::ops::Range<usize>) -> String {
    n.map(|i| para(tag, i)).collect()
}

/// A project of five files: `\input` files (`common.tex` read twice), an
/// `\include`d chapter, a file read inside a `\parbox`; errors inside
/// macros defined in `main.tex` and in `sec1.tex` (the body), an overfull
/// box, an undefined reference, an alignment error.
fn multi_file() -> Vec<(&'static str, String)> {
    let main = "\\documentclass{article}\n\
                \\newcommand{\\inner}[1]{#1 \\undefinedthing}\n\
                \\newcommand{\\wrap}[1]{\\inner{#1}}\n\
                \\begin{document}\n\
                \\input{sec1}\n\
                \\input{common}\n\
                \\include{sec2}\n\
                \\input{common}\n\
                \\noindent\\parbox{6cm}{\\input{boxed}}\n\
                \\end{document}\n"
        .to_string();
    let sec1 = format!(
        "\\section{{First}}\\label{{first}}\n{}Here \\wrap{{x}} fails inside two macros.\n\n{}\
         \\def\\late{{\\undefinedlate}}\n\
         \\hbox to 20pt{{An overfull box in the first input}}\n\n{}",
        paras("First", 0..8),
        paras("First", 8..12),
        paras("First", 12..20)
    );
    let sec2 = format!(
        "\\section{{Second}}\n{}Uses \\late{{}} here.\n\n{}See \\ref{{nolabel}}.\n\n\
         Math error x^2 here.\n\n{}\\begin{{tabular}}{{ll}} a & b \\\\ c & d & e \\\\ \\end{{tabular}}\n\n",
        paras("Second", 0..6),
        paras("Second", 6..11),
        paras("Second", 11..20)
    );
    let common = format!(
        "{}A common error \\foo{{}} in a file read twice.\n\n{}",
        paras("Common", 0..2),
        paras("Common", 2..3)
    );
    let boxed = format!(
        "{}Boxed text with \\wrap{{y}} in it.\n\n{}",
        paras("Boxed", 0..2),
        paras("Boxed", 2..3)
    );
    vec![
        ("main.tex", main),
        ("sec1.tex", sec1),
        ("sec2.tex", sec2),
        ("common.tex", common),
        ("boxed.tex", boxed),
    ]
}

#[test]
fn spans_and_diagnostics_across_edits_in_a_multi_file_project() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let p = |t: &str, i: usize| para(t, i);
    let steps = vec![
        step(
            "sec1.tex",
            &p("First", 1),
            &format!("% a comment line\n{}", p("First", 1)),
            "a line inserted before the error and the \\def",
        ),
        step(
            "sec2.tex",
            &p("Second", 3),
            &format!("{}A new paragraph.\n\n", p("Second", 3)),
            "a paragraph inserted before the errors",
        ),
        step(
            "sec1.tex",
            &p("First", 2),
            "",
            "two lines deleted before the error",
        ),
        step(
            "main.tex",
            "\\include{sec2}",
            "% c\n\\include{sec2}",
            "a line inserted in main",
        ),
        step(
            "sec2.tex",
            "Uses \\late{} here.",
            "Uses \\relax{} here.",
            "an error fixed",
        ),
        step(
            "sec2.tex",
            "Uses \\relax{} here.",
            "Uses \\late{} here.",
            "the error back",
        ),
        step(
            "common.tex",
            &p("Common", 1),
            &format!("{}% two\n% lines\n", p("Common", 1)),
            "lines inserted in the file read twice",
        ),
        step(
            "boxed.tex",
            &p("Boxed", 0),
            &format!("% one\n{}", p("Boxed", 0)),
            "a line inserted in the boxed file",
        ),
        step(
            "sec1.tex",
            "First paragraph 9 has enough plain words",
            "First paragraph 9 has enough plain wurds",
            "a word after the error",
        ),
        step(
            "sec1.tex",
            &format!("\n\n{}", &p("First", 10)[..20]),
            &format!(" {}", &p("First", 10)[..20]),
            "two paragraphs joined",
        ),
        step(
            "main.tex",
            "{#1 \\undefinedthing}",
            "{#1 \\undefinedthing\\relax}",
            "the inner macro redefined",
        ),
    ];
    let bad = audit("multi", &multi_file(), "main.tex", &steps, &[]);
    assert!(
        bad.is_empty(),
        "{} differences:\n{}",
        bad.len(),
        bad.join("\n")
    );
}

/// Calls BOX-MEMO records in an edit's window and replays at the next
/// keystroke (BOX-MEMO.md §6, §11.4): `\measure`, a measure call that
/// throws its box away and defines `\lastw` (`\xdef`), whose expansion is
/// an error on the same page; and `\drawit`, a draw call in an `\mbox`,
/// whose glyphs carry the call's span. Typing on the call's line replays
/// both; a line inserted above them moves their spans.
#[test]
fn spans_and_diagnostics_of_replayed_calls_equal_a_fresh_run() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let mut main = String::from(
        "\\documentclass{article}\n\
         \\newcommand{\\measure}[1]{{\\setbox0\\hbox{#1}\\xdef\\lastw{\\the\\wd0 \\noexpand\\undefinedw}}}\n\
         \\newcommand{\\drawit}[1]{\\kern1pt #1\\kern1pt}\n\
         \\begin{document}\n",
    );
    main.push_str(&paras("Lead", 0..12));
    main.push_str(
        "Typed text \\measure{some words} here, \\mbox{\\drawit{drawn words}} and \\lastw{} after.\n\n",
    );
    main.push_str(&paras("Tail", 0..12));
    main.push_str("\\end{document}\n");
    let typed = |n: usize| format!("Typed{} text", "x".repeat(n));
    let mut steps = vec![];
    for n in 0..4 {
        steps.push(step(
            "main.tex",
            &typed(n),
            &typed(n + 1),
            "a letter typed on the calls' line",
        ));
    }
    steps.push(step(
        "main.tex",
        &para("Lead", 11),
        &format!("{}% a comment line\n", para("Lead", 11)),
        "a line inserted above the calls",
    ));
    for n in 4..6 {
        steps.push(step(
            "main.tex",
            &typed(n),
            &typed(n + 1),
            "a letter typed on the calls' moved line",
        ));
    }
    let (bad, hits) = audit_counting(
        "boxmemo",
        &[("main.tex", main)],
        "main.tex",
        &steps,
        &[("FLASHTEX_BOXMEMO_NAMES", "measure,drawit")],
    );
    assert!(
        bad.is_empty(),
        "{} differences:\n{}",
        bad.len(),
        bad.join("\n")
    );
    // (both calls, at two keystrokes or more; 6 measured on the PC)
    eprintln!("BOX-MEMO hits: {hits}");
    assert!(hits >= 4, "the calls were replayed {hits} times");
}

/// A longer two-chapter project with hyperref (links carry spans), errors
/// in footnotes and inside a macro, UTF-8 text before an error on its line
/// (columns are bytes), and edits that converge early so that later pages
/// and their notes are kept: a word made longer (byte offsets move, lines
/// do not), lines inserted and deleted, two files edited in one compile,
/// and two edits in one file in one compile.
#[test]
fn kept_pages_and_their_diagnostics_follow_edits_with_hyperref() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let main = "\\documentclass{article}\n\
                \\usepackage[T1]{fontenc}\n\
                \\usepackage{hyperref}\n\
                \\newcommand{\\bad}[1]{\\textbf{#1}\\nothere}\n\
                \\begin{document}\n\
                \\tableofcontents\n\
                \\input{ch1}\n\
                \\input{ch2}\n\
                \\end{document}\n"
        .to_string();
    let chapter = |tag: &str, other: &str| {
        let mut s = format!("\\section{{{tag}}}\\label{{sec:{tag}}}\n");
        for i in 0..36 {
            s.push_str(&para(tag, i));
            match i % 9 {
                2 => s.push_str(&format!(
                    "Naïve café text before \\bad{{{tag}{i}}} on one line.\n\n"
                )),
                4 => s.push_str(&format!(
                    "A footnote\\footnote{{With \\nofoot{{}} in {tag} {i}.}} here, \
                     see Section~\\ref{{sec:{other}}} and \\href{{https://example.org/{i}}}{{a link}}.\n\n"
                )),
                6 => s.push_str(&format!("\\subsection{{Part {i} of {tag}}}\n")),
                _ => {}
            }
        }
        s
    };
    let files = vec![
        ("main.tex", main),
        ("ch1.tex", chapter("One", "Two")),
        ("ch2.tex", chapter("Two", "One")),
    ];
    let p = |t: &str, i: usize| para(t, i);
    let all = vec![
        step(
            "ch1.tex",
            "One paragraph 1 has enough plain words",
            "One paragraph 1 has enough plainer words",
            "a word made longer early in ch1",
        ),
        step(
            "ch1.tex",
            &p("One", 3),
            &format!("% one\n% two\n{}", p("One", 3)),
            "two lines inserted early in ch1",
        ),
        step(
            "ch2.tex",
            &p("Two", 20),
            &format!("% a line late in ch2\n{}", p("Two", 20)),
            "a line inserted late in ch2",
        ),
        steps(
            &[
                ("ch1.tex", "% one\n% two\n", ""),
                ("ch2.tex", "% a line late in ch2\n", ""),
            ],
            "lines deleted in two files in one compile",
        ),
        steps(
            &[
                ("ch1.tex", &p("One", 30), &format!("% x\n{}", p("One", 30))),
                ("ch1.tex", &p("One", 5), &format!("{}% y\n", p("One", 5))),
            ],
            "two lines inserted in one file in one compile, the later first",
        ),
        step(
            "ch1.tex",
            "With \\nofoot{} in One 13.",
            "With \\relax{} in One 13.",
            "a footnote's error fixed",
        ),
        step(
            "ch1.tex",
            "Naïve café text before \\bad{One11}",
            "Naïve café, naïve text before \\bad{One11}",
            "UTF-8 text inserted before an error on its line",
        ),
    ];
    let bad = audit("hyperref", &files, "main.tex", &all, &[]);
    assert!(
        bad.is_empty(),
        "{} differences:\n{}",
        bad.len(),
        bad.join("\n")
    );
}
