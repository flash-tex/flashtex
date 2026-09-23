//! `\usepackage{relsize}` runs the real `relsize.sty` (v4.1, vendored as
//! `crate::packages::RELSIZE_STY`) in the expansion engine: it reads the
//! class's size commands and `\f@size`, and the parser receives the size
//! command it picks (plus `\fontsize..\selectfont` past its tolerance).
//!
//! Expected values are pdflatex's (TeX Live 2026), measured with
//! `\typeout{\f@size/\f@baselineskip/\meaning\@currsize}` inside each
//! group of the same documents.
use flashtex_compiler::parser::{parse, Block, ExplicitSize, FontSizeLevel as L, Inline};

/// The body's cases, one group each, tagged by a single word.
const CASES: &[(&str, &str)] = &[
    ("larger", "\\larger"),
    ("smaller", "\\smaller"),
    ("largertwo", "\\larger[2]"),
    ("smallertwo", "\\smaller[2]"),
    ("smalllarger", "\\small\\larger"),
    ("footnotelarger", "\\footnotesize\\larger"),
    ("tinylarger", "\\tiny\\larger"),
    ("tinysmaller", "\\tiny\\smaller"),
    ("Hugelarger", "\\Huge\\larger"),
    ("Largesmaller", "\\Large\\smaller"),
    ("relsizeminusthree", "\\relsize{-3}"),
    ("relsizehalf", "\\relsize{0.5}"),
    ("relscaleup", "\\relscale{1.5}"),
    ("relscaledown", "\\relscale{0.8}"),
    ("largerthrice", "\\larger\\larger\\larger"),
    ("hugesmaller", "\\huge\\smaller"),
    ("scriptlarger", "\\scriptsize\\larger"),
];

fn document(options: &str, preamble: &str) -> String {
    let body: String = CASES.iter().map(|(tag, cmds)| format!("{{{cmds} {tag}}}\n")).collect();
    format!("\\documentclass[{options}]{{article}}\n{preamble}\\usepackage{{relsize}}\n\\begin{{document}}\nnormal\n{body}\\end{{document}}\n")
}

/// `(word, size)` for every text run of the first paragraph.
fn sizes(source: &str) -> Vec<(String, Option<L>)> {
    let parsed = parse(source);
    let errors: Vec<_> = parsed.diagnostics.iter().filter(|d| d.severity == flashtex_compiler::diagnostics::Severity::Error).map(|d| &d.message).collect();
    assert!(errors.is_empty(), "{errors:?}");
    let mut out = Vec::new();
    for block in &parsed.blocks {
        if let Block::Paragraph(inlines) = block {
            for inline in inlines {
                if let Inline::Text { text, style, .. } = inline {
                    for word in text.split_whitespace() {
                        out.push((word.to_string(), style.size));
                    }
                }
            }
        }
    }
    out
}

fn check(options: &str, preamble: &str, expected: &[Option<L>]) {
    let got = sizes(&document(options, preamble));
    let mut want = vec![("normal".to_string(), None)];
    want.extend(CASES.iter().zip(expected).map(|((tag, _), size)| (tag.to_string(), *size)));
    assert_eq!(got, want, "{options} {preamble}");
}

#[test]
fn ten_point_steps_pick_the_closest_class_size() {
    use L::*;
    check(
        "10pt",
        "",
        &[
            Some(Large1),
            Some(FootnoteSize),
            Some(Large2),
            Some(ScriptSize),
            None,
            None,
            Some(Tiny),
            Some(Tiny),
            Some(Huge2),
            Some(Large1),
            Some(Tiny),
            None,
            Some(Large2),
            Some(FootnoteSize),
            Some(Large3),
            Some(Large3),
            Some(FootnoteSize),
        ],
    );
}

#[test]
fn eleven_point_steps_start_from_ten_point_ninety_five() {
    use L::*;
    check(
        "11pt",
        "",
        &[
            Some(Large1),
            Some(FootnoteSize),
            Some(Large2),
            Some(ScriptSize),
            Some(Large1),
            None,
            Some(ScriptSize),
            Some(Tiny),
            Some(Huge2),
            Some(Large1),
            Some(Tiny),
            Some(Large1),
            Some(Large3),
            Some(FootnoteSize),
            Some(Large3),
            Some(Large3),
            Some(Small),
        ],
    );
}

/// `size12.clo` has `\let\Huge=\huge`: relsize's size list records only
/// `\huge` at 24.88pt, so `\Huge\larger` lands on `\huge`.
#[test]
fn twelve_point_huge_larger_is_huge() {
    use L::*;
    check(
        "12pt",
        "",
        &[
            Some(Large1),
            Some(FootnoteSize),
            Some(Large2),
            Some(ScriptSize),
            None,
            None,
            Some(ScriptSize),
            Some(Tiny),
            Some(Huge1),
            Some(Large1),
            Some(Tiny),
            None,
            Some(Large2),
            Some(FootnoteSize),
            Some(Large3),
            Some(Large3),
            Some(FootnoteSize),
        ],
    );
}

/// Latin Modern's shapes cover size ranges, so relsize's tolerance is 5%
/// and a pick more than 5% off the target adds `\fontsize{<target>}{<1.2
/// target>}\selectfont` (pdflatex `\f@size/\f@baselineskip` in comments).
#[test]
fn latin_modern_adjusts_past_five_percent() {
    use L::*;
    let explicit = |size: f64, skip: f64| {
        let sp = |pt: f64| (pt * 65536.0).round() as i32;
        Some(Explicit(ExplicitSize { size_sp: sp(size), font_sp: sp(size), baselineskip_sp: sp(skip) }))
    };
    let got = sizes(&document("10pt", "\\usepackage{lmodern}\n"));
    let size_of = |tag: &str| got.iter().find(|(word, _)| word == tag).map(|(_, size)| *size).unwrap();
    // Named picks within 5% stay named.
    assert_eq!(size_of("larger"), Some(Large1));
    assert_eq!(size_of("relscaleup"), Some(Large2));
    // 10.79997/12.95993pt, 5.99998/7.19995pt, 5.78706/6.94446pt,
    // 10.95444/13.14528pt.
    for (tag, size, skip) in [
        ("smalllarger", 10.79997, 12.95993),
        ("tinylarger", 5.99998, 7.19995),
        ("relsizeminusthree", 5.78706, 6.94446),
        ("relsizehalf", 10.95444, 13.14528),
    ] {
        let Some(Explicit(got)) = size_of(tag) else { panic!("{tag}: {:?}", size_of(tag)) };
        let Some(Explicit(want)) = explicit(size, skip) else { unreachable!() };
        assert!((got.size_sp - want.size_sp).abs() <= 1, "{tag}: {got:?} vs {want:?}");
        assert!((got.baselineskip_sp - want.baselineskip_sp).abs() <= 1, "{tag}: {got:?} vs {want:?}");
    }
}

/// `\textlarger`/`\textsmaller`/`\textscale` scope the step to their
/// argument.
#[test]
fn text_forms_scope_their_argument() {
    let got = sizes("\\documentclass{article}\n\\usepackage{relsize}\n\\begin{document}\na \\textlarger{b} c \\textsmaller[2]{d} e \\textscale{1.2}{f} g\n\\end{document}\n");
    let size_of = |tag: &str| got.iter().find(|(word, _)| word == tag).map(|(_, size)| *size).unwrap();
    assert_eq!(size_of("b"), Some(L::Large1));
    assert_eq!(size_of("d"), Some(L::ScriptSize));
    assert_eq!(size_of("f"), Some(L::Large1));
    for plain in ["a", "c", "e", "g"] {
        assert_eq!(size_of(plain), None, "{plain}");
    }
}

/// With relsize loaded the engine runs the size commands
/// (`\protected\def\small{..}`); they still reach the parser as themselves,
/// including the environment form.
#[test]
fn size_commands_keep_their_environment_form() {
    let got = sizes("\\documentclass{article}\n\\usepackage{relsize}\n\\begin{document}\na \\begin{small}b\\end{small} c {\\Large d} e\n\\end{document}\n");
    let size_of = |tag: &str| got.iter().find(|(word, _)| word == tag).map(|(_, size)| *size).unwrap();
    assert_eq!(size_of("b"), Some(L::Small));
    assert_eq!(size_of("d"), Some(L::Large2));
    for plain in ["a", "c", "e"] {
        assert_eq!(size_of(plain), None, "{plain}");
    }
}

/// Loading the package reports nothing: the size list parses.
#[test]
fn loading_is_silent() {
    let parsed = parse("\\documentclass{article}\n\\usepackage{relsize}\n\\begin{document}\nx\n\\end{document}\n");
    let messages: Vec<_> = parsed.diagnostics.iter().map(|d| &d.message).collect();
    assert!(messages.is_empty(), "{messages:?}");
}
