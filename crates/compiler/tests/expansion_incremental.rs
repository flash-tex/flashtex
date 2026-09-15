//! The incremental expansion cache (`expansion::expand_project_with_cache`,
//! used behind `parser::parse_project` for large entry documents) must give
//! exactly what a from-scratch `expansion::expand_project` gives: the same
//! parser tokens with the same spans and definition spans, the same
//! diagnostics, and the same `\arraystretch` records, after every edit.
use flashtex_compiler::expansion::{expand_project, expand_project_with_cache, ExpansionCache};
use flashtex_compiler::parser::SourceDocument;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// About 20 KB exercising every construct the pass treats specially.
fn document() -> String {
    let mut s = String::from(
        "\\documentclass{article}\n\\newcommand{\\proj}{FlashTeX}\n\\newcommand{\\pair}[2]{(#1, #2)}\n\
         \\def\\twice#1{#1#1}\n\\newenvironment{note}{\\begin{center}}{\\end{center}}\n\\begin{document}\n",
    );
    for i in 0..40 {
        s.push_str(&format!(
            "\\section{{Part {i}}}\nParagraph {i} uses \\proj{{}} and \\pair{{a}}{{b{i}}} and \\twice{{x}}. % comment {i}\n\
             Math $x^{{{i}}} + \\alpha$ and $$\\frac{{a}}{{b}}$$\n\n\
             \\begin{{itemize}}\\item one \\item two\\end{{itemize}}\n\
             \\verb|raw%{{{i}}}| and \\url{{http://x.org/a%20b{i}}}\n\
             \\begin{{verbatim}}\n code {{{i}}} % not a comment\n\\end{{verbatim}}\n\
             {{\\bfseries grouped}} \\renewcommand{{\\arraystretch}}{{1.{i}}}\n\
             \\begin{{tabular}}{{ll}} a & b \\\\ c & d \\end{{tabular}}\n\
             \\begin{{note}}Noted {i}\\end{{note}}\n\n"
        ));
    }
    s.push_str("\\end{document}\n");
    s
}

/// Compare cached and full expansion; returns whether the full expansion ran
/// into the step limit or the output token limit.
fn check(text: &str, cache: &mut Option<ExpansionCache>, step: usize, what: &str) -> bool {
    check_project(&[SourceDocument { path: "main.tex", text }], cache, step, what)
}

/// [`check`] for a project whose entry is its first document.
fn check_project(docs: &[SourceDocument<'_>], cache: &mut Option<ExpansionCache>, step: usize, what: &str) -> bool {
    let full = expand_project(docs, 0);
    let cached = expand_project_with_cache(docs, 0, cache);
    if *cached.tokens != *full.tokens {
        let at = cached
            .tokens
            .iter()
            .zip(full.tokens.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(cached.tokens.len().min(full.tokens.len()));
        panic!(
            "step {step} ({what}): tokens differ at {at} (cached {} vs full {} tokens)\ncached: {:?}\nfull:   {:?}",
            cached.tokens.len(),
            full.tokens.len(),
            cached.tokens.get(at),
            full.tokens.get(at)
        );
    }
    if cached.diagnostics != full.diagnostics {
        let at = cached
            .diagnostics
            .iter()
            .zip(full.diagnostics.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(cached.diagnostics.len().min(full.diagnostics.len()));
        panic!(
            "step {step} ({what}): diagnostics differ at {at} (cached {} vs full {})\n  cached: {:?}\n  full:   {:?}",
            cached.diagnostics.len(),
            full.diagnostics.len(),
            cached.diagnostics.get(at),
            full.diagnostics.get(at)
        );
    }
    if cached.arraystretch != full.arraystretch {
        let mut only_cached: Vec<_> = cached.arraystretch.iter().filter(|(k, v)| full.arraystretch.get(*k) != Some(*v)).collect();
        let mut only_full: Vec<_> = full.arraystretch.iter().filter(|(k, v)| cached.arraystretch.get(*k) != Some(*v)).collect();
        only_cached.sort();
        only_full.sort();
        panic!("step {step} ({what}): arraystretch differs\n  cached only: {only_cached:?}\n  full only:   {only_full:?}");
    }
    full.diagnostics
        .iter()
        .any(|d| d.message.contains("expansion step limit exceeded") || d.message == "output token limit exceeded")
}

fn boundary(text: &str, rng: &mut Rng) -> usize {
    let mut at = rng.below(text.len() + 1);
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

#[test]
fn cached_expansion_matches_full_expansion_under_random_edits() {
    const PIECES: &[&str] = &[
        "a", "Z", " ", "\n", "\n\n", "\\", "{", "}", "%", "$", "#", "~", "é", "\\foo", "\\proj", "\\section{X}",
        "\\verb|q%|", "\\def\\y{Y}", "\\renewcommand{\\arraystretch}{2}", "\\begin{tabular}{l}a\\\\b\\end{tabular}",
        "\\url{a%b}", "\\begin{verbatim}\nv\n\\end{verbatim}\n", "\\newcommand{\\proj}{P}", "\\iffalse", "\\fi",
    ];
    let mut text = document();
    let mut cache = None;
    let mut rng = Rng(0x5EED_CAFE_F00D_0001);
    check(&text, &mut cache, 0, "initial");
    for step in 1..=300 {
        let before = text.clone();
        let what = if rng.below(3) == 0 && text.len() > 64 {
            let start = boundary(&text, &mut rng);
            let mut end = (start + 1 + rng.below(24)).min(text.len());
            while !text.is_char_boundary(end) {
                end += 1;
            }
            let removed = text[start..end].to_string();
            text.replace_range(start..end, "");
            format!("delete {removed:?} at {start}")
        } else {
            let at = boundary(&text, &mut rng);
            let piece = PIECES[rng.below(PIECES.len())];
            text.insert_str(at, piece);
            format!("insert {piece:?} at {at}")
        };
        // A runaway document is checked once, then the edit is undone so the
        // walk keeps exercising normal edits.
        if check(&text, &mut cache, step, &what) {
            text = before;
        }
    }
}

#[test]
fn typing_and_line_deletion_match_full_expansion() {
    let base = document();
    let mut cache = None;
    let mut text = base.clone();
    check(&text, &mut cache, 0, "initial");
    // Type a word and spaces inside a mid-document paragraph, one byte per step.
    let at = base.len() / 2 + base[base.len() / 2..].find("uses").expect("paragraph");
    for (step, byte) in "abcde fghij ".bytes().enumerate() {
        text.insert(at + step, byte as char);
        check(&text, &mut cache, step + 1, "typing");
    }
    // Delete and restore a whole line, then a line holding a definition.
    for needle in ["Math $x^{7}", "\\newcommand{\\pair}"] {
        let start = text.find(needle).expect("line");
        let end = start + text[start..].find('\n').expect("newline") + 1;
        let line = text[start..end].to_string();
        text.replace_range(start..end, "");
        check(&text, &mut cache, 100, "delete line");
        text.insert_str(start, &line);
        check(&text, &mut cache, 101, "restore line");
    }
}

#[test]
fn repeated_engine_diagnostics_match_full_expansion() {
    // Identical engine reports collapse between safe points; edits that add,
    // split and remove repeats must still give the full expansion's list.
    const PIECES: &[&str] = &["\\bad ", "\\bad\\bad ", "\n", " ", "\\relax ", "\\count1=\\relax ", "x", "\\fi"];
    let mut text = String::from("\\documentclass{article}\n\\newcommand{\\bad}{\\ifnum\\relax<1 \\fi\\ifnum\\relax<1 \\fi}\n\\begin{document}\n");
    for i in 0..300 {
        text.push_str(&format!("Paragraph {i} \\bad{{}} text.\n"));
    }
    text.push_str("\\end{document}\n");
    let mut cache = None;
    let mut rng = Rng(0x5EED_D1A6_0000_0001);
    check(&text, &mut cache, 0, "initial");
    for step in 1..=120 {
        let before = text.clone();
        let at = boundary(&text, &mut rng);
        let what = if rng.below(3) == 0 {
            let mut end = (at + 1 + rng.below(20)).min(text.len());
            while !text.is_char_boundary(end) {
                end += 1;
            }
            text.replace_range(at..end, "");
            format!("delete at {at}")
        } else {
            let piece = PIECES[rng.below(PIECES.len())];
            text.insert_str(at, piece);
            format!("insert {piece:?} at {at}")
        };
        if check(&text, &mut cache, step, &what) {
            text = before;
        }
    }
}

/// A runaway loop is re-expanded incrementally like any other edit (no full
/// re-expansion after the stop): typing before, inside and after it, and
/// deleting it, must still give the full expansion's tokens (including the
/// unexpanded rest of the document after the stop) and diagnostics. Typing
/// changes the step limit, which grows with the document.
#[test]
fn edits_around_a_runaway_loop_match_full_expansion() {
    let base = document();
    let mut cache = None;
    let loop_at = base.find("\\section{Part 20}").expect("section");
    let mut text = base.clone();
    text.insert_str(loop_at, "\\def\\r{x\\r}\\r ");
    let mut runaway = 0;
    runaway += check(&text, &mut cache, 0, "insert loop") as usize;
    let place = |text: &str, needle: &str, offset: usize| text.find(needle).expect("needle") + offset;
    let edits: [(&str, usize, &str); 6] = [
        ("\\section{Part 3}", 0, "a"),
        ("\\section{Part 3}", 0, "b "),
        ("\\section{Part 30}", 0, "c"),
        ("{x\\r}", 1, "y"),
        ("\\section{Part 30}", 0, "%"),
        ("\\documentclass", 0, "\n"),
    ];
    for (step, (needle, offset, piece)) in edits.into_iter().enumerate() {
        let at = place(&text, needle, offset);
        text.insert_str(at, piece);
        runaway += check(&text, &mut cache, step + 1, &format!("insert {piece:?} at {at}")) as usize;
    }
    // Same-length replacements keep the limits, so these re-runs converge
    // with the stopped run and reuse its suffix up to the stop.
    for (step, (from, to)) in [("Paragraph 5 ", "Paragraph 6 "), ("Paragraph 6 ", "Paragraph 5 ")].into_iter().enumerate() {
        let at = text.find(from).expect("paragraph");
        text.replace_range(at..at + from.len(), to);
        runaway += check(&text, &mut cache, step + 7, &format!("replace {from:?} at {at}")) as usize;
    }
    assert_eq!(runaway, 9, "every revision runs away");
    let start = text.find("\\def\\r").expect("loop");
    let end = start + text[start..].find("\\r ").expect("call") + 3;
    text.replace_range(start..end, "");
    assert!(!check(&text, &mut cache, 9, "delete loop"));
}

/// The limits count every project document's bytes, so a document the entry
/// does not include still changes them.
#[test]
fn a_runaway_entry_follows_the_size_of_other_project_documents() {
    let mut main = document();
    main.insert_str(main.find("\\section{Part 20}").expect("section"), "\\def\\r{x\\r}\\r ");
    let mut cache = None;
    for (step, bytes) in [0usize, 64, 64, 4096, 1].into_iter().enumerate() {
        let other = "y".repeat(bytes);
        let docs = [SourceDocument { path: "main.tex", text: &main }, SourceDocument { path: "other.tex", text: &other }];
        assert!(check_project(&docs, &mut cache, step, &format!("other.tex {bytes} bytes")));
    }
}

/// The full path stops on the output token limit exactly where the
/// incremental expander does: a loop that emits text runs into that limit
/// before the step limit.
#[test]
fn an_output_heavy_loop_stops_both_paths_at_the_output_token_limit() {
    let mut text = document();
    text.insert_str(text.find("\\section{Part 20}").expect("section"), "\\def\\o{xyzw xyzw xyzw\\o}\\o ");
    let full = expand_project(&[SourceDocument { path: "main.tex", text: &text }], 0);
    let stops: Vec<&str> = full.diagnostics.iter().map(|d| d.message.as_str()).filter(|m| m.contains("limit exceeded")).collect();
    assert_eq!(stops, ["output token limit exceeded"]);
    let mut cache = None;
    assert!(check(&text, &mut cache, 0, "output-heavy loop"));
    let at = text.find("\\section{Part 3}").expect("section");
    text.insert_str(at, "typed ");
    assert!(check(&text, &mut cache, 1, "typing before the loop"));
}

/// [`cached_expansion_matches_full_expansion_under_random_edits`] with loops
/// that fill the output: each loop revision is checked, edited once more
/// (while it runs away, unless the loop landed in a comment, a verbatim
/// block or an argument), then undone.
#[test]
fn output_heavy_runaway_loops_match_full_expansion_under_random_edits() {
    const PIECES: &[&str] = &["a", " ", "\n", "\n\n", "{", "}", "\\proj", "\\def\\y{Y}", "$x$", "%"];
    const LOOPS: &[&str] = &[
        "\\def\\o{xyzw xyzw xyzw\\o}\\o ",
        "\\def\\o{ab\\o}\\o ",
        "\\def\\o{{x}$y$\\o}\\o ",
        "\\newcommand{\\oo}{\\proj{} and \\oo}\\oo ",
        "\\def\\o#1{#1#1\\o{#1}}\\o{q}",
        "\\def\\o{\\begin{note}n\\end{note}\\o}\\o ",
    ];
    let mut text = document();
    let mut cache = None;
    let mut rng = Rng(0x5EED_0070_7B75_0001);
    check(&text, &mut cache, 0, "initial");
    let mut runaways = 0;
    for step in 1..=32 {
        let at = boundary(&text, &mut rng);
        if rng.below(2) == 0 {
            let piece = PIECES[rng.below(PIECES.len())];
            text.insert_str(at, piece);
            check(&text, &mut cache, step, &format!("insert {piece:?} at {at}"));
            continue;
        }
        let before = text.clone();
        let piece = LOOPS[rng.below(LOOPS.len())];
        text.insert_str(at, piece);
        let what = format!("insert {piece:?} at {at}");
        runaways += check(&text, &mut cache, step, &what) as usize;
        let at = boundary(&text, &mut rng);
        let typed = PIECES[rng.below(PIECES.len())];
        text.insert_str(at, typed);
        check(&text, &mut cache, step, &format!("{what}, then insert {typed:?} at {at}"));
        text = before;
        check(&text, &mut cache, step, "undo the loop");
    }
    assert!(runaways >= 6, "only {runaways} runaway revisions");
}
