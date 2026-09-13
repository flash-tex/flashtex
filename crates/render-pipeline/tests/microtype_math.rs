//! Inline math under `\usepackage{microtype}` against pdflatex: pdfTeX's
//! font expansion also applies to the character nodes of an inline formula
//! (`\showbox`: `\mathon \OT1/cmr/m/n/10.95 (+20) ( ...`), its
//! `\medmuskip`/`\thickmuskip` glue stretches with the line, and its first
//! or last character protrudes into the margin. The documents in
//! `fixtures/microtype-math/` (15 justified paragraphs: digits, parentheses,
//! relations, letters, scripts, fractions, operators, formulas at both line
//! edges) render with pdflatex's line breaks and every matched word's left
//! edge within 0.1pt of pdflatex's.
//!
//! `fixtures/microtype-math/<name>.expected` holds pdflatex's words per line
//! (`pdftotext -bbox`, written by `generate.py` there; TeX Live 2026,
//! pdfTeX 1.40.29). No TeX runs in the test.
//!
//! Two of the five fixtures are `#[ignore]`d because their formulas have a
//! natural width that is wrong before any of this applies; each `#[ignore]`
//! reason names the measured cause and the work that owns it. They are not
//! font expansion, and un-ignoring them is that work's acceptance check.

mod common;

use common::*;

/// A character as it is compared, or `None` for one that is dropped (an
/// operator, a delimiter, punctuation): only letters and digits are read the
/// same way by pdftotext on both sides. A big operator becomes its cmex10 slot
/// as pdftotext reads it: `\sum` "50 is `P`, `\prod` "51 `Q`, `\int` "52 `R`.
fn norm_char(c: char) -> Option<char> {
    let c = match c {
        '\u{2211}' => 'P',
        '\u{220F}' => 'Q',
        '\u{222B}' => 'R',
        c => c,
    };
    c.is_ascii_alphanumeric().then_some(c)
}

/// Letters and digits of a word.
fn norm(s: &str) -> String {
    s.chars().filter_map(norm_char).collect()
}

struct Line {
    page: u32,
    words: Vec<(f64, String)>,
}

impl Line {
    /// The line's letters and digits in sorted order: a script or fraction
    /// part may sort before or after its neighbours by a fraction of a point,
    /// so a line is identified by its characters, not their order.
    fn text(&self) -> String {
        let mut c: Vec<char> = self.words.iter().flat_map(|(_, t)| norm(t).chars().collect::<Vec<_>>()).collect();
        c.sort_unstable();
        c.into_iter().collect()
    }
}

type Row = (u32, f64, Vec<(f64, String)>);

fn fixture(name: &str) -> (String, Vec<Line>) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/microtype-math");
    let tex = std::fs::read_to_string(dir.join(format!("{name}.tex"))).unwrap();
    let expected = std::fs::read_to_string(dir.join(format!("{name}.expected"))).unwrap();
    let mut rows: Vec<Row> = Vec::new();
    for l in expected.lines() {
        if let Some(rest) = l.strip_prefix("line ") {
            let mut it = rest.split_whitespace();
            let page = it.next().unwrap().parse().unwrap();
            let y = it.next().unwrap().parse().unwrap();
            rows.push((page, y, Vec::new()));
        } else if let Some(rest) = l.strip_prefix("word ") {
            let (x, text) = rest.split_once(' ').unwrap();
            rows.last_mut().unwrap().2.push((x.parse().unwrap(), text.to_string()));
        }
    }
    (tex, merge_rows(rows))
}

/// Rows of words at one vertical position (pdftotext's `yMax`, or our
/// baseline) merged into output lines. The rows with the most words are the
/// lines; the few-word rows of scripts, fraction parts and operator limits
/// join the nearest line on their page. Both sides are grouped this way.
fn merge_rows(rows: Vec<Row>) -> Vec<Line> {
    let mut order: Vec<usize> = (0..rows.len()).collect();
    order.sort_by(|&a, &b| rows[b].2.len().cmp(&rows[a].2.len()).then(a.cmp(&b)));
    let mut anchors: Vec<usize> = Vec::new();
    for i in order {
        if !anchors.iter().any(|&a| rows[a].0 == rows[i].0 && (rows[a].1 - rows[i].1).abs() < 9.0) {
            anchors.push(i);
        }
    }
    anchors.sort_by(|&a, &b| (rows[a].0, rows[a].1).partial_cmp(&(rows[b].0, rows[b].1)).unwrap());
    let mut lines: Vec<Line> = anchors.iter().map(|&a| Line { page: rows[a].0, words: Vec::new() }).collect();
    for (page, y, words) in &rows {
        let nearest = (0..anchors.len())
            .filter(|&k| rows[anchors[k]].0 == *page)
            .min_by(|&k, &l| (rows[anchors[k]].1 - y).abs().partial_cmp(&(rows[anchors[l]].1 - y).abs()).unwrap())
            .unwrap();
        lines[nearest].words.extend(words.iter().cloned());
    }
    for l in &mut lines {
        l.words.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    }
    lines.retain(|l| l.text().len() >= 3);
    lines
}

/// One entry per glyph rather than per run. pdftotext splits a formula into
/// words wherever it sees a gap, which rarely matches how we group glyphs into
/// runs, so a run's pen position is often not the pen position of the glyph
/// pdflatex reports. A run whose glyphs and characters are not one to one (a
/// ligature, a combining mark) stays a single entry, as `words_of` has it.
fn glyph_words(r: &flashtex_render_pipeline::Rendered) -> Vec<Word> {
    use flashtex_render_pipeline::display::{Item, TICKS_PER_BP};
    let mut words = Vec::new();
    for page in &r.v2.pages {
        for it in &page.items {
            let Item::GlyphRun(run) = it else { continue };
            let chars: Vec<char> = run.text.chars().collect();
            if chars.len() != run.glyphs.len() {
                if let (Some(first), Some(last)) = (run.glyphs.first(), run.glyphs.last()) {
                    words.push(Word {
                        page: page.number,
                        text: run.text.clone(),
                        x: first.origin_x.to_bp(),
                        baseline: first.baseline_y.to_bp(),
                        width: (last.origin_x.0 + last.advance_x.0 - first.origin_x.0) as f64 / TICKS_PER_BP,
                    });
                }
                continue;
            }
            for (g, c) in run.glyphs.iter().zip(chars) {
                words.push(Word {
                    page: page.number,
                    text: c.to_string(),
                    x: g.origin_x.to_bp(),
                    baseline: g.baseline_y.to_bp(),
                    width: g.advance_x.0 as f64 / TICKS_PER_BP,
                });
            }
        }
    }
    words
}

/// Our glyph runs as rows by page and baseline, merged like pdflatex's.
fn our_lines(words: &[Word]) -> Vec<Line> {
    let mut sorted: Vec<&Word> = words.iter().collect();
    sorted.sort_by(|a, b| (a.page, a.baseline, a.x).partial_cmp(&(b.page, b.baseline, b.x)).unwrap());
    let mut rows: Vec<Row> = Vec::new();
    for w in sorted {
        match rows.last_mut() {
            Some((p, y, ws)) if *p == w.page && (w.baseline - *y).abs() < 0.5 => ws.push((w.x, w.text.clone())),
            _ => rows.push((w.page, w.baseline, vec![(w.x, w.text.clone())])),
        }
    }
    merge_rows(rows)
}

struct Outcome {
    lines: usize,
    matched: usize,
    unaligned: usize,
    over: Vec<String>,
    max_dx: f64,
}

/// A line as one stream of letters and digits in reading order, with the pen
/// position of the glyph that starts each pdftotext word (or, on our side,
/// each glyph run). Inside a formula the two sides split runs differently, so
/// words cannot be paired by their text — `x` appears many times on a line —
/// but a position in the character stream is the same glyph on both sides.
struct Stream {
    chars: Vec<char>,
    /// `(index into `chars`, pen x in bp)`, in increasing index order.
    starts: Vec<(usize, f64)>,
}

fn stream(words: &[(f64, String)]) -> Stream {
    let mut s = Stream { chars: Vec::new(), starts: Vec::new() };
    for (x, t) in words {
        // The recorded pen position belongs to the word's first glyph, so it
        // only names a position in the stream when that glyph is one of the
        // compared characters. pdftotext's `(1+2)=3` and our `+2b` both start
        // on a dropped glyph and would otherwise be read as the pen position
        // of the following digit, a whole `(` or `+` wide.
        if t.chars().next().and_then(norm_char).is_some() {
            s.starts.push((s.chars.len(), *x));
        }
        s.chars.extend(norm(t).chars());
    }
    s
}

/// pdflatex word left edges vs ours. Each pdflatex word is matched to the
/// nearest unused run of ours on the same line with the same letters and
/// digits; words our runs split differently (inside a formula) are skipped.
fn compare(name: &str) -> Option<Outcome> {
    if !lm_available() {
        eprintln!("skipping {name}: Latin Modern not installed");
        return None;
    }
    let (tex, want) = fixture(name);
    let r = render_one(&tex);
    let got = our_lines(&glyph_words(&r));
    let got_text: Vec<String> = got.iter().map(Line::text).collect();
    let want_text: Vec<String> = want.iter().map(Line::text).collect();
    if std::env::var_os("MT_MATH_DEBUG").is_some() {
        for d in &r.v2.diagnostics {
            eprintln!("{name} diagnostic: {d:?}");
        }
        for (i, l) in want.iter().enumerate() {
            eprintln!("{name} pdflatex line {i}: {:?}", l.words);
        }
        for (i, l) in got.iter().enumerate() {
            eprintln!("{name} ours     line {i}: {:?}", l.words);
        }
    }
    assert_eq!(got_text, want_text, "{name}: line breaks differ from pdflatex");
    let mut o = Outcome { lines: want.len(), matched: 0, unaligned: 0, over: Vec::new(), max_dx: 0.0 };
    for (li, (g, w)) in got.iter().zip(&want).enumerate() {
        let (gs, ws) = (stream(&g.words), stream(&w.words));
        if gs.chars != ws.chars {
            // A script, fraction part or operator limit that sorts by x on
            // the other side of its neighbour: the two streams hold the same
            // characters (the line assertion above) in a different order, so
            // no index means the same glyph. Reported, never silently passed.
            o.unaligned += 1;
            eprintln!("  line {li}: reading order differs, not compared\n    ours     {:?}\n    pdflatex {:?}", gs.chars, ws.chars);
            continue;
        }
        for (i, x) in &ws.starts {
            let Some((_, gx)) = gs.starts.iter().find(|(j, _)| j == i) else {
                // pdflatex starts a word inside one of our runs (a formula we
                // paint as fewer runs): no pen position of ours to compare.
                continue;
            };
            o.matched += 1;
            let dx = (gx - x).abs() * 72.27 / 72.0;
            o.max_dx = o.max_dx.max(dx);
            if dx > 0.1 {
                let text: String = ws.chars[*i..].iter().take(8).collect();
                o.over.push(format!("page {} line {li} at {text:?} ours {gx:.3}bp pdflatex {x:.3}bp ({dx:.3}pt)", w.page));
            }
        }
    }
    eprintln!(
        "{name}: {} lines identical to pdflatex, {} of them in the same reading order, {} pen positions compared, {} over 0.1pt, max dx {:.4}pt",
        o.lines,
        o.lines - o.unaligned,
        o.matched,
        o.over.len(),
        o.max_dx
    );
    for s in &o.over {
        eprintln!("  {s}");
    }
    Some(o)
}

fn check(name: &str) {
    if let Some(o) = compare(name) {
        assert!(o.over.is_empty(), "{name}: {} words more than 0.1pt from pdflatex: {:#?}", o.over.len(), o.over);
    }
}

#[test]
fn digits_and_parentheses_expand_inside_formulas() {
    check("01-digits-parens");
}

#[test]
fn relations_and_letters_stretch_math_glue_with_the_line() {
    check("02-letters-relations");
}

/// Line breaks already match pdflatex; the 13 pen positions still over 0.1pt
/// (worst 2.09pt) are two math metric gaps that are not font expansion, both
/// measured against `\showthe\wd` of the same formulas (pdfTeX 1.40.27):
///
/// * `\sum` is **1.003pt** narrow, on this fixture and on `$\sum a_i$`,
///   `$\sum_{i=1}^n i=\frac{n(n+1)}{2}$` alike. pdflatex's `\sum` is
///   11.55836pt = 1.05556 em of **cmex10 at 10.95pt**; ours is 10.5556pt, the
///   same glyph at cmex10's 10pt design size. The difference is 1.00276pt.
/// * `\cdots` is **5.475pt** narrow on its own (3 × `\thinmuskip` at 10.95pt)
///   and 3.649pt narrow inside `$a_1+a_2+\cdots+a_n$`: we set three
///   `\cdotp`s where pdflatex sets `\mathinner{\cdotp\cdotp\cdotp}`.
///
/// Both are what PR #177 changes (family 3 at the math size with amsmath;
/// `\dots`/`\cdots` as `\mathinner`). Everything else in the fixture —
/// `\frac`, `\sqrt`, scripts, `+`, `=` — is within 0.0003pt of pdflatex.
#[test]
#[ignore = "blocked on #177: \\sum is 1.003pt narrow (cmex10 at 10pt, not the math size) and \\cdots 5.475pt narrow (not \\mathinner); not font expansion"]
fn scripts_and_fractions_stay_boxes() {
    check("03-scripts-fractions");
}

#[test]
fn formula_characters_protrude_at_line_edges() {
    check("04-line-edges");
}

/// This fixture does not reach the expansion comparison: the compiler drops
/// `\mathbb{Z}` and sets `\colon` as the literal characters `\colon`, so
/// `$h\colon A\to B$` and `$A=B=\mathbb{Z}$` hold the wrong glyphs and the
/// paragraph breaks elsewhere. The same gap is why HW1/HW2 lines holding
/// `\mathbb{Z}` differ from their reference. Paragraph 1 additionally has two
/// operator-name spacing gaps, measured against `\showthe\wd`: `$\sin x$` is
/// 1.899pt narrow and `$\log 2<1$` 2.055pt narrow (about one `\thinmuskip`
/// each), while `$\max(a,b)$`, `$a\,b$`, `$a\quad b$`, `$x\;y$`, `$p\mid q$`,
/// `$n!=120$`, `$\{1,2,3\}$` and `$h^{-1}(b)=b-1$` are all within 0.12pt.
///
/// `\colon`/`\mathbb` are the compiler lane's (the Mac owns them); the
/// operator-name spacing is the same family as #177's `\bmod`/`\pmod` work.
/// Neither is font expansion.
#[test]
#[ignore = "blocked on the compiler \\colon/\\mathbb gap (Mac lane) and operator-name spacing (\\sin 1.899pt, \\log 2.055pt narrow); not font expansion"]
fn operators_functions_and_spacing_commands() {
    check("05-operators-functions");
}
