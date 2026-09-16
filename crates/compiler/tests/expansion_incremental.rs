//! The incremental expansion cache (`expansion::expand_project_with_cache`,
//! used behind `parser::parse_project` for large entry documents) must give
//! exactly what a from-scratch `expansion::expand_project` gives: the same
//! parser tokens with the same spans and definition spans, the same
//! diagnostics, and the same `\arraystretch`/`\labelitem` records, after
//! every edit.
use flashtex_compiler::expansion::{expand_project, expand_project_with_cache, ExpansionCache};
use flashtex_compiler::parser::{self, Block, SourceDocument};

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
    if cached.labelitem_overrides != full.labelitem_overrides {
        panic!(
            "step {step} ({what}): labelitem_overrides differs\n  cached: {:?}\n  full:   {:?}",
            cached.labelitem_overrides, full.labelitem_overrides
        );
    }
    full.diagnostics
        .iter()
        .any(|d| d.message.contains("expansion step limit exceeded") || d.message.starts_with("TeX capacity exceeded, sorry [output token limit="))
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

/// Itemize labels shown after each parse of one editing session.
fn session_label_texts(path: &str, text: &str) -> Vec<String> {
    let documents = [SourceDocument { path, text }];
    parser::parse_project(&documents, path)
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::ListItem { item, .. } => item.as_ref().map(|item| item.text().to_string()),
            _ => None,
        })
        .collect()
}

#[test]
fn edited_labelitem_override_updates_the_itemize_label_in_the_same_session() {
    // Live-typing repro: editing the captured `\labelitemi` body must change
    // the itemize label on re-parse, not keep showing the stale text. The
    // document is over the incremental threshold so the re-parse goes
    // through the incremental machinery exactly like the editor does (a
    // definition-body edit declines the suffix join, so this pins the
    // end-to-end behavior rather than the splice path itself — see the
    // healed-redefinition test below for splice coverage).
    const PATH: &str = "labelitem-incremental-edit.tex";
    let mut filler = String::new();
    for i in 0..200 {
        filler.push_str(&format!(
            "Paragraph {i} with some ordinary text padding the document beyond the incremental threshold.\n\n"
        ));
    }
    let head = "\\documentclass{article}\n\\begin{document}\n\\renewcommand{\\labelitemi}{X}\\begin{itemize}\\item A\\end{itemize}\n";
    let tail = "\n\\end{document}\n";
    let before = format!("{head}{filler}{tail}");
    assert!(before.len() > 4 * 1024, "repro needs a document over 4 KiB");
    assert_eq!(session_label_texts(PATH, &before), ["X"]);
    let after = before.replacen("\\renewcommand{\\labelitemi}{X}", "\\renewcommand{\\labelitemi}{Y}", 1);
    assert_eq!(
        session_label_texts(PATH, &after),
        ["Y"],
        "editing the override body must update the label, not reuse the stale one"
    );
}

#[test]
fn healed_labelitem_redefinition_reuses_the_cached_suffix() {
    // Only the converter's suffix-splice path restores `labelitem_log`
    // entries, and it only runs once the engine converges downstream. A
    // later identical `\renewcommand` heals the edited definition, so the
    // re-parse converges mid-document and splices the rest: the second
    // itemize sits past the convergence point, so its record can only come
    // from the restored suffix. It must keep the healed text while the
    // re-captured first itemize shows the edited body.
    let mut near = String::new();
    let mut far = String::new();
    for i in 0..200 {
        let paragraph = format!(
            "Paragraph {i} with some ordinary text padding the document beyond the incremental threshold.\n\n"
        );
        if i < 8 {
            near.push_str(&paragraph);
        } else {
            far.push_str(&paragraph);
        }
    }
    let head = "\\documentclass{article}\n\\begin{document}\n\\renewcommand{\\labelitemi}{X}\\begin{itemize}\\item A\\end{itemize}\n\\renewcommand{\\labelitemi}{X}\n";
    let mid = "\\begin{itemize}\\item B\\end{itemize}\n";
    let tail = "\n\\end{document}\n";
    let before = format!("{head}{near}{mid}{far}{tail}");
    assert!(before.len() > 4 * 1024, "repro needs a document over 4 KiB");
    let mut cache = None;
    check(&before, &mut cache, 0, "initial");
    // Edit only the FIRST body, leaving the healer untouched.
    let first = before.find("\\renewcommand{\\labelitemi}{X}").expect("first override");
    let mut after = before.clone();
    after.replace_range(
        first + "\\renewcommand{\\labelitemi}{".len()..first + "\\renewcommand{\\labelitemi}{X".len(),
        "Y",
    );
    check(&after, &mut cache, 1, "edit first body");
    let docs = [SourceDocument { path: "main.tex", text: &after }];
    let cached = expand_project_with_cache(&docs, 0, &mut cache);
    let mut firsts: Vec<&str> = cached
        .labelitem_overrides
        .values()
        .map(|recorded| recorded.texts[0].as_str())
        .collect();
    firsts.sort_unstable();
    assert_eq!(
        firsts,
        ["X", "Y"],
        "the edited itemize must show Y while the healed one keeps X, not two copies of one revision"
    );
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
    let stops: Vec<(&str, Option<&str>)> = full
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("limit exceeded") || d.message.starts_with("TeX capacity exceeded"))
        .map(|d| (d.message.as_str(), d.recovery.as_deref()))
        .collect();
    let limit = 2_000_000 + 8 * text.len();
    assert_eq!(
        stops,
        [(
            format!("TeX capacity exceeded, sorry [output token limit={limit}]; expansion stopped here and the rest of the document was not typeset.").as_str(),
            Some("stopped expanding; the rest of the document was not typeset")
        )]
    );
    // As the message says, nothing after the loop is typeset.
    let after_loop = text.find("\\section{Part 20}").expect("section");
    assert!(full.tokens.iter().all(|t| t.token.span.start < after_loop));
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

/// Each expansion limit's recovery note says what the engine does: past a
/// nesting limit it drops the extra `{` or conditional and goes on; after a
/// step-limit stop the rest is typeset unexpanded.
#[test]
fn expansion_limit_notes_describe_what_happens() {
    let note = |text: &str, message: &str| -> Option<String> {
        let expansion = expand_project(&[SourceDocument { path: "main.tex", text }], 0);
        let found = expansion.diagnostics.iter().find(|d| d.message.starts_with(message));
        found.unwrap_or_else(|| panic!("no {message:?} in {:?}", expansion.diagnostics)).recovery.clone()
    };
    let groups = format!("{}x{} after", "{".repeat(10_010), "}".repeat(10_010));
    assert_eq!(note(&groups, "group nesting limit exceeded").as_deref(), Some("the extra group was ignored and expansion continued"));
    let conditionals = format!("{}x{} after", "\\iftrue ".repeat(10_010), "\\fi ".repeat(10_010));
    assert_eq!(
        note(&conditionals, "conditional nesting limit exceeded").as_deref(),
        Some("the extra conditional was ignored without evaluating its test, and expansion continued")
    );
    assert_eq!(
        note("\\def\\r{x\\r}\\r after", "expansion step limit exceeded").as_deref(),
        Some("stopped expanding; the rest of the document was typeset without macro expansion")
    );
}
