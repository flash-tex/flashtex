//! Beamer Tier 2, overlays (issue #944; corpus PR #943,
//! `fixtures/real-world/beamer-overlays`): what the parser hands the
//! render pipeline for a frame with overlay specifications.
//!
//! Oracle: pdflatex (TeX Live 2026) sets the corpus deck as 3 + 3 + 2 + 2
//! + 3 + 1 = 14 pages, one per slide; the frame's slide count is the
//! largest slide any specification names (`beamerbasedecode.sty`
//! `\beamer@anotherslide`), `\pause` being `\onslide<\beamerpauses->` after
//! a step. The frame's material is emitted once with zero-width markers
//! (`Inline::OverlayBegin`/`OverlayEnd`/`Onslide`); the per-slide
//! show/cover/omit decision is the render pipeline's
//! (`crates/render-pipeline/tests/beamer_overlays.rs`).

use flashtex_compiler::overlay::{OverlayKind, OverlayRange, OverlaySpec};
use flashtex_compiler::parser::{parse, Block, Inline};

fn deck(body: &str) -> String {
    format!("\\documentclass{{beamer}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn frame(body: &str) -> String {
    deck(&format!("\\begin{{frame}}\n{body}\n\\end{{frame}}"))
}

fn blocks(text: &str) -> Vec<Block> {
    parse(text).blocks
}

/// The frame's slide count and the marker/word stream of its body, one
/// entry per block: `<BEGIN kind spec>`, `<END>`, `<ONSLIDE spec>` and the
/// words.
fn stream(text: &str) -> (u32, Vec<String>) {
    let parsed = parse(text);
    let errors: Vec<_> = parsed
        .diagnostics
        .iter()
        .filter(|d| d.severity == flashtex_compiler::diagnostics::Severity::Error)
        .map(|d| d.message.clone())
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
    let mut slides = 0;
    let mut out = Vec::new();
    for block in &parsed.blocks {
        let inlines = match block {
            Block::BeamerFrameBegin { slides: n, .. } => {
                slides = *n;
                continue;
            }
            Block::Paragraph(p) => p.as_slice(),
            Block::ListItem { content, .. } => content.as_slice(),
            Block::Styled { content, .. } => content.as_slice(),
            _ => continue,
        };
        out.push(show(inlines));
    }
    (slides, out)
}

fn show(inlines: &[Inline]) -> String {
    inlines
        .iter()
        .filter_map(|i| match i {
            Inline::Text { text, .. } => Some(text.clone()),
            Inline::OverlayBegin { spec, kind, .. } => Some(format!("<BEGIN {kind:?} {}>", spec_text(spec))),
            Inline::OverlayEnd { .. } => Some("<END>".to_string()),
            Inline::Onslide { spec, .. } => Some(format!("<ONSLIDE {}>", spec_text(spec))),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn spec_text(spec: &OverlaySpec) -> String {
    if spec.all {
        return "*".to_string();
    }
    spec.ranges
        .iter()
        .map(|r| match r {
            OverlayRange::One(n) => n.to_string(),
            OverlayRange::From(n) => format!("{n}-"),
            OverlayRange::Until(n) => format!("-{n}"),
            OverlayRange::Between(a, b) => format!("{a}-{b}"),
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// `beamerbasedecode.sty`: the specification grammar, as the corpus and
/// the beamer manual (§9.2) use it.
#[test]
fn overlay_spec_grammar() {
    let table: &[(&str, &[u32], &[u32], u32)] = &[
        // spec, selected, not selected, largest slide named
        ("1-", &[1, 2, 9], &[], 1),
        ("2-", &[2, 3], &[1], 2),
        ("2-3", &[2, 3], &[1, 4], 3),
        ("1,3", &[1, 3], &[2, 4], 3),
        ("-2", &[1, 2], &[3], 2),
        ("*", &[1, 7], &[], 0),
        ("", &[1, 7], &[], 0),
        ("2-|handout:1", &[2, 5], &[1], 2),
        ("beamer:2|all:4", &[2, 4], &[1, 3], 4),
        ("handout:2", &[1, 2, 3], &[], 0),
        ("1-|alert@2", &[1, 2], &[], 1),
        ("2, 4", &[2, 4], &[1, 3], 4),
    ];
    for (raw, yes, no, max) in table {
        let mut pauses = 1;
        let spec = OverlaySpec::parse(raw, &mut pauses);
        for s in *yes {
            assert!(spec.contains(*s), "<{raw}> should select slide {s}: {spec:?}");
        }
        for s in *no {
            assert!(!spec.contains(*s), "<{raw}> should not select slide {s}: {spec:?}");
        }
        assert_eq!(spec.max_slide(), *max, "<{raw}>");
        assert_eq!(pauses, 1, "<{raw}> has no `+`");
    }
}

/// `+` reads `beamerpauses` and steps it once per specification after the
/// decode; `.` reads one less and never steps.
#[test]
fn plus_and_dot_track_the_pause_counter() {
    let mut pauses = 1;
    let a = OverlaySpec::parse("+-", &mut pauses);
    assert_eq!((pauses, spec_text(&a)), (2, "1-".to_string()));
    let b = OverlaySpec::parse("+-", &mut pauses);
    assert_eq!((pauses, spec_text(&b)), (3, "2-".to_string()));
    let c = OverlaySpec::parse(".-", &mut pauses);
    assert_eq!((pauses, spec_text(&c)), (3, "2-".to_string()));
    let d = OverlaySpec::parse("+(1)-", &mut pauses);
    assert_eq!((pauses, spec_text(&d)), (4, "4-".to_string()));
    let e = OverlaySpec::parse("+-+(1)", &mut pauses);
    assert_eq!((pauses, spec_text(&e)), (5, "4-5".to_string()));
}

/// `\pause`: `\onslide<k->` after the k-th pause; the frame sets
/// pauses + 1 slides. The marker rides in the item's paragraph.
#[test]
fn pause_is_onslide_from_the_next_slide() {
    let (slides, s) = stream(&frame(
        "Intro.\n\\begin{itemize}\n\\item One.\n\\pause\n\\item Two.\n\\pause\n\\item Three.\n\\end{itemize}",
    ));
    assert_eq!(slides, 3);
    assert_eq!(s, vec!["Intro.", "One. <ONSLIDE 2->", "Two. <ONSLIDE 3->", "Three."]);
    // `\pause[n]` sets the counter.
    let (slides, s) = stream(&frame("A \\pause[4] B"));
    assert_eq!((slides, s), (4, vec!["A <ONSLIDE 4-> B".to_string()]));
}

/// `\item<2->` brackets the item (label included: the marker opens the
/// item's paragraph) up to the next `\item` or the list's end
/// (`\beamer@closeitem`); the slide count is the largest number named.
#[test]
fn item_overlays_bracket_each_item() {
    let (slides, s) = stream(&frame(
        "\\begin{itemize}\n\\item<1-> A.\n\\item<2-> B.\n\\item<3-> C.\n\\item<2-> D.\n\\item E.\n\\end{itemize}",
    ));
    assert_eq!(slides, 3);
    assert_eq!(
        s,
        vec![
            "<BEGIN Cover 1-> A. <END>",
            "<BEGIN Cover 2-> B. <END>",
            "<BEGIN Cover 3-> C. <END>",
            "<BEGIN Cover 2-> D. <END>",
            "E.",
        ]
    );
    // `\item[label]<spec>` and `\item<spec>[label]` both read.
    let (slides, s) = stream(&frame("\\begin{itemize}\n\\item[x]<2> A.\n\\item<3>[y] B.\n\\end{itemize}"));
    assert_eq!(slides, 3);
    assert_eq!(s, vec!["<BEGIN Cover 2> A. <END>", "<BEGIN Cover 3> B. <END>"]);
}

/// `\begin{itemize}[<+->]`: every item without its own specification takes
/// `<+->`, each decode stepping the counter, so the items uncover in turn
/// and the frame has as many slides as items.
#[test]
fn itemize_default_spec_increments_per_item() {
    let (slides, s) = stream(&frame("\\begin{itemize}[<+->]\n\\item A.\n\\item B.\n\\item C.\n\\end{itemize}"));
    assert_eq!(slides, 3);
    assert_eq!(s, vec!["<BEGIN Cover 1-> A. <END>", "<BEGIN Cover 2-> B. <END>", "<BEGIN Cover 3-> C. <END>"]);
    // An explicit specification on an item wins over the default and the
    // `+` of `<+->` on the *next* item still counts from the pause counter.
    let (slides, s) = stream(&frame("\\begin{itemize}[<+->]\n\\item A.\n\\item<1-> B.\n\\item C.\n\\end{itemize}"));
    assert_eq!(slides, 2);
    assert_eq!(s, vec!["<BEGIN Cover 1-> A. <END>", "<BEGIN Cover 1-> B. <END>", "<BEGIN Cover 2-> C. <END>"]);
}

/// The braced commands bracket their argument; `\alert<spec>` keeps the
/// surrounding colour in the compiler (the slide decides), while `\alert`
/// without a specification stays red on every slide, as before.
#[test]
fn braced_commands_bracket_their_argument() {
    let (slides, s) = stream(&frame(
        "\\only<1>{First}\\only<2>{Second}\n\nThe \\alert<2>{word}: \\uncover<2->{hidden at first}.\n\n\\visible<3>{v} \\invisible<2>{i} \\onslide<2>{o}",
    ));
    assert_eq!(slides, 3);
    assert_eq!(
        s,
        vec![
            "<BEGIN Only 1> First <END> <BEGIN Only 2> Second <END>",
            "The <BEGIN Alert 2> word <END> : <BEGIN Cover 2-> hidden at first <END> .",
            "<BEGIN Visible 3> v <END> <BEGIN Invisible 2> i <END> <BEGIN Cover 2> o <END>",
        ]
    );
    let parsed = parse(&frame("\\alert{always} \\alert<2>{sometimes}"));
    let colours: Vec<(String, bool)> = parsed
        .blocks
        .iter()
        .filter_map(|b| match b {
            Block::Paragraph(p) => Some(p),
            _ => None,
        })
        .flatten()
        .filter_map(|i| match i {
            Inline::Text { text, style, .. } => Some((text.clone(), style.color.is_some())),
            _ => None,
        })
        .collect();
    assert_eq!(colours, vec![("always".to_string(), true), ("sometimes".to_string(), false)]);
}

/// The specification may follow the argument (`\only{...}<2>`), and an
/// argument holding a paragraph break or a list leaves its `END` in the
/// later block: the markers straddle blocks, the pipeline walks them in
/// order.
#[test]
fn markers_straddle_blocks() {
    let (slides, s) = stream(&frame("\\uncover<2->{First para.\n\nSecond para.} After."));
    assert_eq!(slides, 2);
    assert_eq!(s, vec!["<BEGIN Cover 2-> First para.", "Second para. <END> After."]);
    let (slides, s) = stream(&frame("\\only{Late spec}<3> tail"));
    assert_eq!(slides, 3);
    assert_eq!(s, vec!["<BEGIN Only 3> Late spec <END> tail"]);
    let (slides, s) = stream(&frame("\\only<2>{\\begin{itemize}\\item In list.\\end{itemize}}\n\nAfter."));
    assert_eq!(slides, 2);
    assert_eq!(s, vec!["<BEGIN Only 2> In list.", "<END> After."]);
}

/// `\onslide<spec>` without braces is a running marker; a `\pause` or
/// `\onslide` standing alone between blank lines opens no paragraph of
/// its own (the marker moves to the next paragraph) and one after the last
/// paragraph is dropped.
#[test]
fn bare_onslide_and_lone_markers() {
    let (slides, s) = stream(&frame("First.\n\\onslide<2->\nSecond.\n\\begin{itemize}\n\\item Bullet.\n\\end{itemize}\n\\onslide<3->\nThird."));
    assert_eq!(slides, 3);
    assert_eq!(s, vec!["First. <ONSLIDE 2-> Second.", "Bullet.", "<ONSLIDE 3-> Third."]);
    let (slides, s) = stream(&frame("First.\n\n\\pause\n\nSecond.\n\n\\pause"));
    assert_eq!(slides, 3);
    assert_eq!(s, vec!["First.", "<ONSLIDE 2-> Second."]);
    let b = blocks(&frame("First.\n\n\\pause\n\nSecond."));
    assert_eq!(b.iter().filter(|b| matches!(b, Block::Paragraph(_))).count(), 2, "{b:?}");
}

/// The overlay environments and the `block` family's specification.
#[test]
fn overlay_environments_and_block_specs() {
    let (slides, s) = stream(&frame(
        "\\begin{uncoverenv}<2->\nA.\n\\end{uncoverenv}\n\\begin{onlyenv}<3>\nB.\n\\end{onlyenv}\n\\begin{alertenv}<2>\nC.\n\\end{alertenv}",
    ));
    assert_eq!(slides, 3);
    assert_eq!(s, vec!["<BEGIN Cover 2-> A. <END> <BEGIN Only 3> B. <END> <BEGIN Alert 2> C. <END>"]);
    // `\begin{block}<2->{Title}` covers the block (title block and body)
    // on slide 1 like `uncoverenv`: the marker rides into the body's
    // paragraph; the title itself lives in the `BeamerBlockBegin` block.
    let (slides, s) = stream(&frame("\\begin{block}<2->{Title}\nBody.\n\\end{block}"));
    assert_eq!(slides, 2);
    assert_eq!(s, vec!["<BEGIN Cover 2-> Body."]);
}

/// A frame without any specification sets one slide; the counters restart
/// per frame.
#[test]
fn plain_frames_have_one_slide_and_counters_restart() {
    let text = deck("\\begin{frame}\nA \\pause B\n\\end{frame}\n\\begin{frame}\nPlain.\n\\end{frame}\n\\begin{frame}\n\\begin{itemize}[<+->]\\item X\\end{itemize}\n\\end{frame}");
    let counts: Vec<u32> = blocks(&text)
        .iter()
        .filter_map(|b| match b {
            Block::BeamerFrameBegin { slides, .. } => Some(*slides),
            _ => None,
        })
        .collect();
    assert_eq!(counts, vec![2, 1, 1]);
}

/// The corpus deck: 3 + 3 + 2 + 2 + 3 + 1 slides, as the comments in
/// `main.tex` say and pdflatex's 14 pages show.
#[test]
fn corpus_deck_slide_counts() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/real-world/beamer-overlays/main.tex")).unwrap();
    let counts: Vec<u32> = blocks(&text)
        .iter()
        .filter_map(|b| match b {
            Block::BeamerFrameBegin { slides, .. } => Some(*slides),
            _ => None,
        })
        .collect();
    assert_eq!(counts, vec![3, 3, 2, 2, 3, 1]);
    assert_eq!(counts.iter().sum::<u32>(), 14);
}

/// Outside beamer the commands are undefined (class-gated like `\alert`).
#[test]
fn overlay_commands_are_class_gated() {
    let parsed = parse("\\documentclass{article}\n\\begin{document}\n\\uncover<2>{x} \\pause y\n\\end{document}\n");
    let gated: Vec<&str> = parsed
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("defined by the beamer document class"))
        .map(|d| d.message.as_str())
        .collect();
    assert_eq!(gated.len(), 2, "{:?}", parsed.diagnostics);
    assert!(parsed.blocks.iter().all(|b| match b {
        Block::Paragraph(p) => !p.iter().any(|i| matches!(i, Inline::OverlayBegin { .. } | Inline::Onslide { .. })),
        _ => true,
    }));
    let _ = OverlayKind::Cover;
}
