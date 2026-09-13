//! The LaTeX kernel's twelve `\DeclareMathDelimiter`s and the kernel/amsmath
//! math commands that went with them, measured through this compiler.
//!
//! The class each command's atom takes is measured the way pdfTeX's own
//! `\showthe\wd0` measures it, but through this compiler so the glyph width
//! cancels: `$a\sym b$` minus `$a\mathord{\sym}b$` is exactly the inter-atom
//! glue the *class* adds, because `\mathord{...}` forces Ord (0 glue beside
//! ordinaries) while leaving the same glyph in the same font. The difference
//! is then compared against a symbol of the declared class the compiler
//! already carried, so the numbers are relative and free of this compiler's
//! own font metrics.
//!
//! The reference classes are `fontmath.ltx` (TeX Live 2025) 457-505 for the
//! delimiters and 253-269/334/374-399/422-423 for the rest, each cited on its
//! row. The pdfTeX cross-check that those classes are right lives in
//! `crates/math-layout/tests/kernel_delimiters.rs`, which pins the measured
//! `\hbox{$\sym$}` and `\hbox{$a\sym b$}` boxes at 10pt.

use flashtex_compiler::diagnostics::{Diagnostic, Severity};
use flashtex_compiler::math::{self, DelimiterRole, Nucleus};

const SIZE: f64 = 12.0;

fn compile(source: &str) -> (math::MathBox, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let list = math::parse_tokens(&flashtex_compiler::lexer::tokenize(source), &mut diagnostics);
    let b = math::layout(&list, SIZE, &mut diagnostics);
    (b, diagnostics)
}

fn width(source: &str) -> f64 {
    let (b, d) = compile(source);
    assert!(d.iter().all(|x| x.severity != Severity::Error), "{source}: {d:?}");
    b.width
}

/// The glue `command`'s class adds between two ordinaries, with the glyph's
/// own width cancelled by the `\mathord` control.
fn class_glue(command: &str) -> f64 {
    width(&format!("a{command} b")) - width(&format!("a\\mathord{{{command}}} b"))
}

/// Every command this lane adds, with a command of the same declared class
/// the compiler already carried. A wrong class moves `class_glue`; the
/// reference command makes the expected value this compiler's own, so the
/// test does not re-encode the mu-to-point conversion.
const SAME_CLASS: &[(&str, &str, &str)] = &[
    // fontmath.ltx 465/469/483: ordinaries on cmsy "6B/"6A/"6E, the slots
    // `\parallel`, `\mid` and `\setminus` hold with other classes.
    ("\\Vert", "\\infty", "fontmath.ltx:465"),
    ("\\vert", "\\infty", "fontmath.ltx:469"),
    ("\\backslash", "\\infty", "fontmath.ltx:483"),
    // 461-464: the extension bars, ordinaries like the two above.
    ("\\arrowvert", "\\infty", "fontmath.ltx:461"),
    ("\\Arrowvert", "\\infty", "fontmath.ltx:463"),
    ("\\bracevert", "\\infty", "fontmath.ltx:505"),
    // 475/481: the two vertical arrow delimiters are relations.
    ("\\updownarrow", "\\equiv", "fontmath.ltx:475"),
    ("\\Updownarrow", "\\equiv", "fontmath.ltx:481"),
    // 501/503 and 457/459: openings and closings.
    ("\\lgroup", "\\langle", "fontmath.ltx:501"),
    ("\\lmoustache", "\\langle", "fontmath.ltx:457"),
    ("\\rgroup", "\\rangle", "fontmath.ltx:503"),
    ("\\rmoustache", "\\rangle", "fontmath.ltx:459"),
    // 377/391: `\leftarrow\joinrel\rhook` and `\mapstochar\longrightarrow`,
    // both relations.
    ("\\hookleftarrow", "\\equiv", "fontmath.ltx:377"),
    ("\\longmapsto", "\\equiv", "fontmath.ltx:391"),
    // 253/260/263: large operators.
    ("\\intop", "\\sum", "fontmath.ltx:253"),
    ("\\ointop", "\\sum", "fontmath.ltx:260"),
    ("\\smallint", "\\sum", "fontmath.ltx:263"),
    // 268-269: binaries, the same two glyphs `\bigtriangleup`/
    // `\bigtriangledown` set.
    ("\\varbigtriangleup", "\\ast", "fontmath.ltx:268"),
    ("\\varbigtriangledown", "\\ast", "fontmath.ltx:269"),
    // 334: `\not` is a relation, which is why `\not=` has a relation's
    // spacing on both sides and none between the two atoms.
    ("\\not", "\\equiv", "fontmath.ltx:334"),
];

/// `\ldotp`/`\cdotp` (fontmath.ltx 398-399) are the only two punctuation
/// marks of the set, and the compiler carried no `\mathpunct` symbol command
/// to compare them with, so they are measured against `,` — which
/// `fontmath.ltx` 152 declares `\mathpunct` and this compiler already sets
/// that way.
const PUNCT: &[&str] = &["\\ldotp", "\\cdotp"];

#[test]
fn every_new_command_takes_its_declared_class() {
    for (command, reference, source) in SAME_CLASS {
        let got = class_glue(command);
        let want = class_glue(reference);
        assert!(
            (got - want).abs() < 1e-6,
            "{command} ({source}): class glue {got:.5} != {reference}'s {want:.5}"
        );
    }
    for command in PUNCT {
        let got = class_glue(command);
        let want = class_glue(",");
        assert!(
            (got - want).abs() < 1e-6,
            "{command} (fontmath.ltx:398-399): class glue {got:.5} != ','s {want:.5}"
        );
    }
}

#[test]
fn a_wrong_class_would_be_caught() {
    // The control on the control: two different classes must not agree, or
    // the test above would pass for any of them.
    let ord = class_glue("\\infty");
    let rel = class_glue("\\equiv");
    let op = class_glue("\\sum");
    let bin = class_glue("\\ast");
    let punct = class_glue(",");
    for (a, b) in [(ord, rel), (ord, op), (ord, bin), (ord, punct), (rel, op), (op, bin)] {
        assert!((a - b).abs() > 1e-3, "two classes measure the same: {a} {b}");
    }
}

#[test]
fn nothing_in_the_set_is_diagnosed_unsupported() {
    let mut sources: Vec<String> = SAME_CLASS
        .iter()
        .map(|(c, ..)| format!("a{c} b"))
        .chain(PUNCT.iter().map(|c| format!("a{c} b")))
        .collect();
    // The shapes that are not `$a\sym b$`.
    sources.extend(
        [
            r"\left\Vert x \right\Vert",
            r"\left\vert x \right\vert",
            r"\left\backslash x \right\backslash",
            r"\left\updownarrow x \right\Updownarrow",
            r"\left\lgroup x \right\rgroup",
            r"\left\lmoustache x \right\rmoustache",
            r"\left\arrowvert x \right\Arrowvert",
            r"\left\bracevert x \right\bracevert",
            r"\not= b",
            r"\mathring{a}",
            r"\sqrtsign{x}",
            r"\injlim_{n} a",
            r"\projlim_{n} a",
            r"\varinjlim a",
            r"\varprojlim a",
            r"\iiiint",
            r"\idotsint",
            r"a\pod{b}",
            r"a\nobreakdash-b",
            r"\lvert a\rvert",
            r"\lVert a\rVert",
        ]
        .iter()
        .map(|s| s.to_string()),
    );
    for source in &sources {
        let (_, diagnostics) = compile(source);
        for d in &diagnostics {
            assert!(
                !d.message.contains("is not supported in math mode"),
                "{source}: {}",
                d.message
            );
            assert!(d.severity != Severity::Error, "{source}: {}", d.message);
        }
    }
}

/// `\pmb` is the one command of the set that is deliberately approximate, and
/// it says so: the advance is the argument's own (which is what pdfTeX
/// measures, since the three overprinted copies cancel), the emboldening is
/// not reproduced, and a warning names that.
#[test]
fn pmb_sets_its_argument_and_warns() {
    let (b, diagnostics) = compile(r"\pmb{x}");
    assert!(diagnostics.iter().any(|d| d.message.contains("\\pmb")));
    assert!(diagnostics.iter().all(|d| d.severity != Severity::Error));
    assert!((b.width - width("x")).abs() < 1e-9, "\\pmb{{x}} is x's width");
}

/// The twelve delimiters resolve to the character `math-layout`'s
/// `cm::delimiter_slot` keys the small/large pair by, both bare and as a
/// fence. Emitting a look-alike instead would leave the fence without a
/// growth chain, which is exactly the bug `\Vert` had: it was spelled as two
/// `\mid` bars, so `\left\Vert` was two atoms and never grew.
#[test]
fn delimiters_resolve_to_the_character_the_layout_tables_key_by() {
    const EXPECTED: &[(&str, &str)] = &[
        (r"\Vert", "\u{2016}"),
        (r"\vert", "\u{2223}"),
        (r"\backslash", "\u{2216}"),
        (r"\updownarrow", "\u{2195}"),
        (r"\Updownarrow", "\u{21D5}"),
        (r"\lgroup", "\u{27EE}"),
        (r"\rgroup", "\u{27EF}"),
        (r"\lmoustache", "\u{23B0}"),
        (r"\rmoustache", "\u{23B1}"),
        (r"\arrowvert", "\u{23D0}"),
        (r"\Arrowvert", "\u{F8FD}"),
        (r"\bracevert", "\u{23AA}"),
    ];
    for (command, glyph) in EXPECTED {
        let mut diagnostics = Vec::new();
        let list = math::parse_tokens(
            &flashtex_compiler::lexer::tokenize(command),
            &mut diagnostics,
        );
        assert_eq!(list.atoms.len(), 1, "{command}");
        match &list.atoms[0].nucleus {
            Nucleus::Symbol(s) => assert_eq!(s, glyph, "{command} bare"),
            other => panic!("{command}: {other:?}"),
        }
        // The same character after `\left`, so the fence grows.
        let mut diagnostics = Vec::new();
        let list = math::parse_tokens(
            &flashtex_compiler::lexer::tokenize(&format!("\\left{command} x \\right.")),
            &mut diagnostics,
        );
        let fence = list
            .atoms
            .iter()
            .find_map(|a| match &a.nucleus {
                Nucleus::SizedDelimiter { glyph, role: DelimiterRole::Left, .. } => {
                    Some(glyph.clone())
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("{command}: no fence atom"));
        assert_eq!(&fence, glyph, "{command} as a \\left fence");
    }
    // `\|` is plain.tex 973's `\let\|=\Vert`, and `\left||` is the same one
    // delimiter, not two bars.
    assert_eq!(
        math::parse_tokens(
            &flashtex_compiler::lexer::tokenize(r"\left\| x \right\|"),
            &mut Vec::new()
        )
        .atoms
        .iter()
        .filter(|a| matches!(&a.nucleus, Nucleus::SizedDelimiter { glyph, .. } if glyph == "\u{2016}"))
        .count(),
        2
    );
}
