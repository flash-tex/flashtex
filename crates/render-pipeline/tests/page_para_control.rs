//! Page and paragraph control commands (GH-64) as the paragraph and page
//! builders see them, against pdflatex.
//!
//! The compiler reports each command as a node (`Inline::Penalty`,
//! `Inline::PagePenalty`, `Inline::Discretionary`, `Block::Penalty`) or as a
//! `Parsed::parameters` / `Parsed::hyphenation` entry; before that every one
//! of them was an unknown command whose number or bracket was typeset as
//! text, and `\-` was painted as a hyphen.
//!
//! Oracle: pdflatex, TeX Live 2026, `article` 10pt with `\pagestyle{empty}`
//! (no TeX runs here). Each expectation below is the line pdflatex set,
//! read from its PDF with PyMuPDF (words grouped by baseline), or its page
//! count. Each engineered case was checked to *discriminate*: the same
//! document without the command sets different lines or pages in pdflatex
//! (the control noted beside it).

mod common;

use common::*;

/// The words of each line, in reading order, per page.
fn lines(rendered: &flashtex_render_pipeline::Rendered) -> Vec<(u32, String)> {
    let mut words = words_of(rendered);
    words.sort_by(|a, b| {
        (a.page, (a.baseline * 100.0) as i64, (a.x * 100.0) as i64)
            .cmp(&(b.page, (b.baseline * 100.0) as i64, (b.x * 100.0) as i64))
    });
    let mut out: Vec<(u32, i64, Vec<String>)> = Vec::new();
    // Runs that abut (no interword glue between them: the two sides of an
    // unbroken discretionary) are one word, as PyMuPDF reads pdflatex's.
    let mut end = f64::MIN;
    for word in words {
        let key = (word.baseline * 100.0) as i64;
        match out.last_mut() {
            Some((page, k, line)) if *page == word.page && *k == key && (word.x - end).abs() < 0.05 => {
                line.last_mut().unwrap().push_str(&word.text)
            }
            Some((page, k, line)) if *page == word.page && *k == key => line.push(word.text.clone()),
            _ => out.push((word.page, key, vec![word.text.clone()])),
        }
        end = word.x + word.width;
    }
    // A pre-break hyphen is its own run; pdflatex's text has it on the word.
    out.into_iter().map(|(page, _, line)| (page, line.join(" ").replace(" -", "-").replace(" =", "="))).collect()
}

fn doc(pre: &str, body: &str) -> String {
    format!("\\documentclass{{article}}\n{pre}\\pagestyle{{empty}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn text_lines(body: &str) -> Vec<String> {
    lines(&render_one(&doc("", body))).into_iter().map(|(_, l)| l).collect()
}

const TIE: &str = "xx xx xx xx xx aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj kkkk llll Figure";

/// Control (`tests/tie.rs`): with a plain space pdflatex ends line 1 with
/// `llll Figure`. A penalty of 10000 before the space's glue, or the
/// space moved behind one, makes that break illegal.
#[test]
fn nobreak_nolinebreak_and_penalty_10000_forbid_the_break() {
    if !lm_available() {
        return;
    }
    for sep in ["\\nobreak\\ ", " \\nolinebreak ", " \\nolinebreak[2] ", "\\penalty10000\\ "] {
        let got = text_lines(&format!("\\noindent {TIE}{sep}7 shows the rest of it."));
        assert_eq!(got, [format!("{TIE} 7"), "shows the rest of it.".to_string()], "{sep:?}");
    }
}

/// Control: `Figure/seven/shows` has no break after the slash and pdflatex
/// ends line 1 with `llll` (hyphenating nothing).
#[test]
fn allowbreak_is_a_legal_break() {
    if !lm_available() {
        return;
    }
    let got = text_lines(&format!("\\noindent {TIE}/\\allowbreak seven/shows the rest of it."));
    assert_eq!(got, [format!("{TIE}/"), "seven/shows the rest of it.".to_string()]);
}

/// pdflatex: `Short words here\linebreak and ...` ends line 1 at `here`
/// with the line *justified*, its last word ending at x = 477.5 bp (the
/// right margin); `\\` sets the same line ragged, ending at 207.8 bp.
/// `\penalty-10000\ ` is the same break as `\linebreak`.
#[test]
fn linebreak_and_a_forced_penalty_end_a_justified_line() {
    if !lm_available() {
        return;
    }
    let rest = "and the rest of the paragraph continues. The quick brown fox jumps over the lazy dog while seventeen zebras contemplate the philosophical implications of quantum chromodynamics.";
    for (sep, right_bp) in [("\\linebreak ", 477.5), ("\\penalty-10000\\ ", 477.5), ("\\\\ ", 207.8)] {
        let r = render_one(&doc("", &format!("\\noindent Short words here{sep}{rest}")));
        let mut words = words_of(&r);
        words.sort_by(|a, b| (a.baseline, a.x).partial_cmp(&(b.baseline, b.x)).unwrap());
        let first = words[0].baseline;
        let line: Vec<_> = words.iter().filter(|w| w.baseline == first).collect();
        assert_eq!(line.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(), ["Short", "words", "here"], "{sep:?}");
        let end = line.last().map(|w| w.x + w.width).unwrap();
        assert!((end - right_bp).abs() < 0.6, "{sep:?}: line 1 ends at {end:.2} bp, pdflatex {right_bp}");
    }
}

/// `\-` is invisible unless the line breaks there. pdflatex: with two more
/// `xx` in front the unbroken `qwrtzplkjhgfd` starts line 2 (no hyphen);
/// without them line 1 ends `qwrtzp-`. `\discretionary{=}{}{}` breaks the
/// same way with `=` as its pre-break text.
#[test]
fn discretionaries_break_with_their_pre_break_text_and_are_invisible_otherwise() {
    if !lm_available() {
        return;
    }
    let tail = "xx xx xx xx xx aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj kkkk qwrtzp";
    assert_eq!(
        text_lines(&format!("\\noindent xx xx {tail}\\-lkjhgfd shows the rest of it.")),
        [format!("xx xx {}", &tail[..tail.len() - " qwrtzp".len()]), "qwrtzplkjhgfd shows the rest of it.".to_string()]
    );
    assert_eq!(
        text_lines(&format!("\\noindent {tail}\\-lkjhgfd shows the rest of it.")),
        [format!("{tail}-"), "lkjhgfd shows the rest of it.".to_string()]
    );
    assert_eq!(
        text_lines(&format!("\\noindent {tail}\\discretionary{{=}}{{}}{{}}lkjhgfd shows the rest of it.")),
        [format!("{tail}="), "lkjhgfd shows the rest of it.".to_string()]
    );
}

const LONG_WORDS: &str = "\\noindent Supercalifragilistic expialidocious antidisestablishmentarianism floccinaucinihilipilification pneumonoultramicroscopicsilicovolcanoconiosis hippopotomonstrosesquippedaliophobia pseudopseudohypoparathyroidism honorificabilitudinitatibus thyroparathyroidectomized.";

/// pdflatex: the patterns give `floccinaucinihilip-`; with the exception
/// `flocc-inaucinihilipilification` its only break point is too early to
/// use and line 1 ends with the whole word.
#[test]
fn hyphenation_exceptions_replace_the_patterns() {
    if !lm_available() {
        return;
    }
    let plain = lines(&render_one(&doc("", LONG_WORDS)));
    assert!(plain[0].1.ends_with("floccinaucinihilip-"), "{plain:?}");
    let excepted = lines(&render_one(&doc("\\hyphenation{flocc-inaucinihilipilification hippo-potomonstrosesquippedaliophobia}\n", LONG_WORDS)));
    assert_eq!(
        excepted.iter().take(2).map(|(_, l)| l.as_str()).collect::<Vec<_>>(),
        [
            "Supercalifragilistic expialidocious antidisestablishmentarianism floccinaucinihilipilification",
            "pneumonoultramicroscopicsilicovolcanoconiosis hippopotomonstrosesquippedaliophobia"
        ]
    );
}

/// pdflatex: `\sloppy` (and `sloppypar`, `\tolerance=9999`,
/// `\emergencystretch=3em` alone) set the long words `floccinaucini-` /
/// `hilipilification ... hippopotomon-`; the default and `\sloppy\fussy`
/// set `floccinaucinihilip-` / `ilification ... hippopotomonstrosesquippedalio-`.
#[test]
fn sloppy_family_and_breaking_parameters_are_read_where_the_paragraph_ends() {
    if !lm_available() {
        return;
    }
    let body = LONG_WORDS.replace("\\noindent ", "");
    let loose = "Supercalifragilistic expialidocious antidisestablishmentarianism floccinaucini-";
    let tight = "Supercalifragilistic expialidocious antidisestablishmentarianism floccinaucinihilip-";
    for (pre, inline, first) in [
        ("", format!("\\noindent {body}"), tight),
        ("", format!("\\begin{{sloppypar}}\\noindent {body}\\end{{sloppypar}}"), loose),
        ("", format!("\\tolerance=9999 \\noindent {body}"), loose),
        ("", format!("\\emergencystretch=3em \\noindent {body}"), loose),
        ("\\sloppy\\fussy\n", format!("\\noindent {body}"), tight),
        // Restored at the end of its group: the second paragraph is fussy.
        ("", format!("{{\\sloppy \\noindent x\\par}}\\noindent {body}"), tight),
    ] {
        let got = lines(&render_one(&doc(pre, &inline)));
        assert!(got.iter().any(|(_, l)| l == first), "{pre:?} {inline:?}: {got:?}");
    }
}

fn filler(from: usize, to: usize) -> String {
    (from..=to).map(|i| format!("\\noindent Line {i}.\\par\n")).collect()
}

/// `Line k.` of the last line on page 1 and the page count.
fn page_one_end(pre: &str, body: &str) -> (String, usize, usize) {
    let r = render_one(&doc(pre, body));
    let l = lines(&r);
    let first: Vec<_> = l.iter().filter(|(p, _)| *p == l[0].0).collect();
    (first.last().unwrap().1.clone(), first.len(), r.v2.pages.len())
}

/// pdflatex, 46 lines fit the page: `\goodbreak`, `\filbreak` and
/// `\penalty-10000` after `Line 43.` end page 1 there (control: `Line 46.`);
/// `\nobreak`/`\nopagebreak` between `Line 46.` and `Line 47.` move the
/// break back to after `Line 45.`; `\pagebreak[1]` between two short
/// paragraphs is only a hint and the document stays on one page.
#[test]
fn vertical_penalties_choose_the_page_break() {
    if !lm_available() {
        return;
    }
    assert_eq!(page_one_end("", &filler(1, 50)).0, "Line 46.");
    for command in ["\\goodbreak", "\\filbreak", "\\penalty-10000"] {
        let got = page_one_end("", &format!("{}{command}\n{}", filler(1, 43), filler(44, 50)));
        assert_eq!((got.0.as_str(), got.2), ("Line 43.", 2), "{command}");
    }
    for command in ["\\nobreak", "\\nopagebreak"] {
        let got = page_one_end("", &format!("{}{command}\n{}", filler(1, 46), filler(47, 49)));
        assert_eq!((got.0.as_str(), got.2), ("Line 45.", 2), "{command}");
    }
    let r = render_one(&doc("", "\\noindent a\\par\\pagebreak[1]\\noindent b"));
    assert_eq!(r.v2.pages.len(), 1);
}

/// pdflatex: `a\pagebreak{} b` and `a\pagebreak[1] b` inside a paragraph set
/// `a b` on one line of one page (the penalty goes after the line).
#[test]
fn pagebreak_inside_a_paragraph_does_not_break_the_paragraph() {
    if !lm_available() {
        return;
    }
    for body in ["\\noindent a\\pagebreak{} b", "\\noindent a\\pagebreak[1] b"] {
        let r = render_one(&doc("", body));
        assert_eq!(r.v2.pages.len(), 1, "{body}");
        assert_eq!(lines(&r), [(lines(&r)[0].0, "a b".to_string())], "{body}");
    }
    // A forced one ends the page after its line.
    let got = page_one_end("", "\\noindent a\\pagebreak{} b\\par\\noindent c");
    assert_eq!((got.0.as_str(), got.2), ("a b", 2));
}

/// pdflatex, a 4-line paragraph after `Line 43.`/`Line 45.`: page 1 holds
/// 45 lines by default (the widow penalty keeps the last line company),
/// 46 with `\widowpenalty=0`; 45 after `Line 45.` by default (club), 46 with
/// `\clubpenalty=0`; 43 with `\interlinepenalty=10000` or `\samepage`
/// (the paragraph moves whole).
#[test]
fn widow_club_and_interline_penalties_are_read_from_the_document() {
    if !lm_available() {
        return;
    }
    let para = "\\noindent Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma tau upsilon phi chi psi omega. Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma tau upsilon phi chi psi omega.";
    let count = |pre: &str, k: usize| page_one_end("", &format!("{pre}{}{para}", filler(1, k))).1;
    assert_eq!(count("", 43), 45);
    assert_eq!(count("\\widowpenalty=0 ", 43), 46);
    assert_eq!(count("", 45), 45);
    assert_eq!(count("\\clubpenalty=0 ", 45), 46);
    assert_eq!(count("\\interlinepenalty=10000 ", 43), 43);
    assert_eq!(page_one_end("", &format!("{}{{\\samepage {para}\\par}}", filler(1, 43))).1, 43);
}

/// pdflatex, 42 lines and an unbreakable 5-line paragraph: `article` is
/// `\raggedbottom` and page 1's last baseline is 627 (PyMuPDF y); with
/// `\flushbottom` its glue stretches to 675. `book` (two-sided) is the
/// other way round: 674, and 626 with `\raggedbottom`.
#[test]
fn raggedbottom_and_flushbottom_decide_the_page_glue() {
    if !lm_available() {
        return;
    }
    let body = format!(
        "{}{{\\interlinepenalty=10000 \\noindent {}\\par}}",
        filler(1, 42),
        "Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma tau upsilon phi chi psi omega. ".repeat(3)
    );
    let last_baseline = |class: &str, pre: &str| {
        let r = render_one(&format!("\\documentclass{{{class}}}\n{pre}\\pagestyle{{empty}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"));
        let words = words_of(&r);
        let page = words.iter().map(|w| w.page).min().unwrap();
        words.iter().filter(|w| w.page == page).map(|w| w.baseline).fold(f64::MIN, f64::max)
    };
    let (ragged, flush) = (last_baseline("article", ""), last_baseline("article", "\\flushbottom\n"));
    assert!(((flush - ragged).abs() - 48.0).abs() < 1.5, "article: {ragged} -> {flush}");
    let (flush, ragged) = (last_baseline("book", ""), last_baseline("book", "\\raggedbottom\n"));
    assert!(((flush - ragged).abs() - 48.0).abs() < 1.5, "book: {flush} -> {ragged}");
}

/// pdflatex: `a\cleardoublepage b` is 3 pages in `book` (two-sided: an
/// empty page 2 so `b` starts page 3) and 2 in `article`.
#[test]
fn cleardoublepage_starts_an_odd_page_when_two_sided() {
    if !lm_available() {
        return;
    }
    for (class, pages) in [("book", 3), ("article", 2)] {
        let r = render_one(&format!("\\documentclass{{{class}}}\n\\pagestyle{{empty}}\n\\begin{{document}}\n\\noindent a\\cleardoublepage b\n\\end{{document}}\n"));
        assert_eq!(r.v2.pages.len(), pages, "{class}");
    }
}

/// latex.ltx `\clearpage` -> `\@doclearpage`: every deferred float goes
/// out on float pages before the text after it. pdflatex: a `[p]` figure
/// written before `\clearpage` is page 2, `Second.` page 3 (with
/// `\newpage` the figure waits for the document's end). Three 8cm `[t]`
/// figures after `Line 30.`: page 2 `Figure 1`/`Figure 2`, page 3
/// `Figure 3`, page 4 `Line 34.`-`Line 36.` (before: `Line 34.` went on
/// page 2 and the figures after it).
#[test]
fn clearpage_flushes_deferred_floats_onto_float_pages() {
    if !lm_available() {
        return;
    }
    let figure = |place: &str, height: &str, caption: &str| {
        format!("\\begin{{figure}}[{place}]\\centering\\rule{{1cm}}{{{height}}}\\caption{{{caption}}}\\end{{figure}}\n")
    };
    let first_lines = |body: &str| {
        let r = render_one(&doc("", body));
        let mut seen = std::collections::BTreeMap::new();
        for (p, l) in lines(&r) {
            seen.entry(p).or_insert(l);
        }
        seen.into_values().collect::<Vec<_>>()
    };
    let p = format!("\\noindent First.\n{}\\clearpage\n\\noindent Second.", figure("p", "3cm", "Pfloat"));
    assert_eq!(first_lines(&p), ["First.", "Figure 1: Pfloat", "Second."]);
    assert_eq!(first_lines(&p.replace("\\clearpage", "\\newpage")), ["First.", "Second.", "Figure 1: Pfloat"]);
    let t = format!(
        "{}{}{}{}{}\\clearpage\n{}",
        filler(1, 30),
        figure("t", "8cm", "Tone"),
        figure("t", "8cm", "Ttwo"),
        figure("t", "8cm", "Tthree"),
        filler(31, 33),
        filler(34, 36)
    );
    assert_eq!(first_lines(&t), ["Line 1.", "Figure 1: Tone", "Figure 3: Tthree", "Line 34."]);
}

/// `\enlargethispage{<dimen>}` (and `*`): `\insert\@kludgeins{\vskip
/// -<dimen>}` makes `\pagegoal` that much larger on the page the insertion
/// is contributed to. pdflatex, one-line paragraphs (46 fit a page): 48 on
/// page 1 after `\enlargethispage{2\baselineskip}` or `{30pt}`, 41 after
/// `{-5\baselineskip}`, and written after `Line 50.` it is page 2 that
/// holds 49.
#[test]
fn enlargethispage_changes_the_goal_of_its_own_page() {
    if !lm_available() {
        return;
    }
    let per_page = |body: &str| {
        let r = render_one(&doc("", body));
        let mut counts = vec![0usize; r.v2.pages.len()];
        for (p, _) in lines(&r) {
            counts[(p - r.v2.pages[0].number) as usize] += 1;
        }
        counts
    };
    for command in ["\\enlargethispage{2\\baselineskip}", "\\enlargethispage*{2\\baselineskip}"] {
        assert_eq!(per_page(&format!("{command}{}", filler(1, 50))), [48, 2], "{command}");
    }
    assert_eq!(per_page(&format!("{}\\enlargethispage{{30pt}}{}", filler(1, 10), filler(11, 50))), [48, 2]);
    assert_eq!(per_page(&format!("{}\\enlargethispage{{-5\\baselineskip}}{}", filler(1, 10), filler(11, 50))), [41, 9]);
    assert_eq!(per_page(&format!("{}\\enlargethispage{{3\\baselineskip}}{}", filler(1, 50), filler(51, 100))), [46, 49, 5]);
}

/// amsmath `\nobreakdash`: the dashes are boxed, so no discretionary
/// follows them, and `\nobreak` after them. pdflatex: the control
/// paragraph ends a line with `113–`; with `\nobreakdash--` no line ends in
/// a dash. `strongly\nobreakdash-minded` is hyphenated `strong-ly-` (the
/// hyphen is a letter to TeX's hyphenation, `\lccode`\-=`\-`).
#[test]
fn nobreakdash_forbids_the_break_after_its_dashes() {
    if !lm_available() {
        return;
    }
    let amsmath = |body: &str| lines(&render_one(&doc("\\usepackage{amsmath}\n", &format!("\\noindent {body}")))).into_iter().map(|(_, l)| l).collect::<Vec<_>>();
    let pages: String = (0..40).map(|i| format!("pages 1{i}--2{i} and ")).collect();
    let control = amsmath(pages.trim_end());
    assert_eq!(control[2], "19–29 and pages 110–210 and pages 111–211 and pages 112–212 and pages 113–");
    let boxed = amsmath(pages.replace("--", "\\nobreakdash--").trim_end());
    assert_eq!(boxed[2], "19–29 and pages 110–210 and pages 111–211 and pages 112–212 and pages");
    assert_eq!(boxed[3], "113–213 and pages 114–214 and pages 115–215 and pages 116–216 and pages");
    let words = "strongly\\nobreakdash-minded quite\\nobreakdash-well pre\\nobreakdash-war ".repeat(12);
    let hyphen = amsmath(words.trim_end());
    assert_eq!(hyphen.len(), 6, "{hyphen:#?}");
    assert!(hyphen[0].ends_with("pre-war strong-"), "{hyphen:#?}");
    assert!(hyphen[1].starts_with("ly-minded quite-well"), "{hyphen:#?}");
}

/// latex.ltx `\pagebreak` is `\penalty-\@M` with no `\vfil`: under
/// `\flushbottom` the page it ends is `\vbox to\textheight`, stretched.
/// pdflatex, 30 one-line paragraphs: page 1's last baseline is at 675
/// after `\pagebreak` and `\penalty-10000`, 484 after `\newpage`.
#[test]
fn flushbottom_stretches_a_page_ended_by_pagebreak_not_newpage() {
    if !lm_available() {
        return;
    }
    let last_baseline = |command: &str| {
        let r = render_one(&doc("\\flushbottom\n", &format!("{}{command}\n{}", filler(1, 30), filler(31, 32))));
        let words = words_of(&r);
        let page = words.iter().map(|w| w.page).min().unwrap();
        words.iter().filter(|w| w.page == page).map(|w| w.baseline).fold(f64::MIN, f64::max)
    };
    let newpage = last_baseline("\\newpage");
    for command in ["\\pagebreak", "\\penalty-10000"] {
        let stretched = last_baseline(command);
        assert!(((stretched - newpage) - (675.0 - 484.0)).abs() < 1.5, "{command}: {newpage} -> {stretched}");
    }
}
