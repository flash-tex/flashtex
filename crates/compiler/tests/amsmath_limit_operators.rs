//! amsopn's six predeclared operators that are not plain words.
//!
//! `crates/compiler/src/math.rs`'s `OPERATOR_NAMES` is a flat list of single
//! upright words, so six of amsopn's own operators could not live in it and
//! reached the user as `error: \injlim is not supported in math mode` --
//! the same shape of "your correct LaTeX is a typo" report as `\sgn` (#286)
//! and the `letter` class (#316), in the other direction.
//!
//! Every construction below is `amsopn.sty` v2.04's, quoted in
//! `math::AMS_SPACED_OPERATORS` / `AMS_BAR_OPERATORS` /
//! `AMS_ARROW_OPERATORS`, and the geometry named in the comments was
//! measured with `\showbox` under pdfTeX 3.141592653-2.6-1.40.27 (TeX Live
//! 2025) at 10pt. No TeX runs here.

use flashtex_compiler::math::{AtomClass, Frame, MathList, Nucleus};
use flashtex_compiler::parser::{self, Block, Inline};

fn math_of(source: &str) -> MathList {
    let full = format!(
        r"\documentclass{{article}}\usepackage{{amsmath}}\begin{{document}}${source}$\end{{document}}"
    );
    let parsed = parser::parse(&full);
    for block in &parsed.blocks {
        if let Block::Paragraph(inlines) = block {
            for inline in inlines {
                if let Inline::Math { list, .. } = inline {
                    return list.clone();
                }
            }
        }
    }
    panic!("no formula in {source}");
}

/// Diagnostics other than the package-recognition warning every fixture in
/// this file raises.
fn diagnostics(source: &str) -> Vec<String> {
    let full = format!(
        r"\documentclass{{article}}\usepackage{{amsmath}}\begin{{document}}${source}$\end{{document}}"
    );
    parser::parse(&full)
        .diagnostics
        .iter()
        .filter(|d| !d.message.contains("recognised but not implemented"))
        .map(|d| d.message.clone())
        .collect()
}

/// `\injlim` and `\projlim` are `\qopname\relax m{inj\,lim}` /
/// `{proj\,lim}`: one `\mathop` whose body is two upright words with a real
/// 3mu atom between them. pdflatex's `\hbox{$\injlim$}` is `inj`,
/// `\glue 1.66663` (= 3mu at 10pt), `lim`.
#[test]
fn the_two_spaced_operators_are_two_words_around_a_thin_space() {
    for (command, first, second) in [("injlim", "inj", "lim"), ("projlim", "proj", "lim")] {
        let list = math_of(&format!("\\{command}"));
        assert_eq!(list.atoms.len(), 1, "\\{command} is one atom");
        let Nucleus::Operator { body, limits } = &list.atoms[0].nucleus else {
            panic!("\\{command} is not an Operator: {:?}", list.atoms[0].nucleus);
        };
        assert!(limits, "\\{command} is `\\qopname\\relax m`, so it takes limits");
        let nuclei: Vec<&Nucleus> = body.atoms.iter().map(|a| &a.nucleus).collect();
        assert_eq!(nuclei.len(), 3, "\\{command} body: {nuclei:?}");
        assert_eq!(nuclei[0], &Nucleus::Text(first.to_string()));
        assert_eq!(nuclei[2], &Nucleus::Text(second.to_string()));
        match nuclei[1] {
            Nucleus::Space { em, .. } => assert!(
                (em - 3.0 / 18.0).abs() < 1e-12,
                "\\{command}'s `\\,` is {em} em, not 3mu"
            ),
            other => panic!("\\{command} has no thin space: {other:?}"),
        }
        assert_eq!(diagnostics(&format!("\\{command}")), Vec::<String>::new());
    }
}

/// `\varlimsup` is `\@@overline{\hbox{lim}}` and `\varliminf` is
/// `\@@underline{...\hbox{lim}}`, both wrapped in `\mathop`. They reuse
/// this crate's `\overline`/`\underline`, whose rule geometry is already
/// oracle-checked against pdflatex (the `amssymb` corpus fixture
/// `37-over-underline` passes within 0.006 bp).
#[test]
fn the_two_bar_operators_are_lim_under_a_rule() {
    for (command, expected) in [("varlimsup", Frame::Over), ("varliminf", Frame::Under)] {
        let list = math_of(&format!("\\{command}"));
        assert_eq!(list.atoms.len(), 1, "\\{command} is one atom");
        let atom = &list.atoms[0];
        let Nucleus::Framed { body, frame } = &atom.nucleus else {
            panic!("\\{command} is not Framed: {:?}", atom.nucleus);
        };
        assert_eq!(*frame, expected, "\\{command}'s rule side");
        let nuclei: Vec<&Nucleus> = body.atoms.iter().map(|a| &a.nucleus).collect();
        assert_eq!(nuclei, vec![&Nucleus::Text("lim".to_string())]);
        assert_eq!(
            atom.class_override,
            Some(AtomClass::Op),
            "\\{command} is a `\\mathop`"
        );
        assert_eq!(diagnostics(&format!("\\{command}")), Vec::<String>::new());
    }
}

/// All four take `\displaylimits`, which is what `\qopname\relax m` means
/// under amsopn's default `namelimits` option: a display-style script
/// stacks over/under the operator, an inline one sits to the side.
/// `Nucleus::Operator` used not to reach `takes_display_limits` at all, so
/// `\operatorname*{...}_n` was side-set in display too.
#[test]
fn all_four_stack_their_display_scripts() {
    use flashtex_compiler::math;
    let width = |source: &str, display: bool| {
        let list = math_of(source);
        let mut ignored = Vec::new();
        if display {
            math::layout_display(&list, 10.0, &mut ignored).width
        } else {
            math::layout(&list, 10.0, &mut ignored).width
        }
    };
    for command in ["injlim", "projlim", "varlimsup", "varliminf"] {
        let bare = width(&format!("\\{command}"), true);
        let stacked = width(&format!("\\{command}_{{n}}"), true);
        let beside = width(&format!("\\{command}_{{n}}"), false);
        assert!(
            (stacked - bare).abs() < 1e-9,
            "\\{command}_n in display is {stacked} wide, not the bare {bare}: the script did not stack"
        );
        assert!(
            beside > bare,
            "\\{command}_n inline is {beside} wide, not wider than the bare {bare}: the script did not go beside"
        );
    }
    // The same switch, on the construct it was always meant for.
    let bare = width(r"\operatorname*{rank}", true);
    let stacked = width(r"\operatorname*{rank}_{n}", true);
    assert!((stacked - bare).abs() < 1e-9, "\\operatorname* did not stack: {stacked} vs {bare}");
}

/// `\varinjlim`/`\varprojlim` are not log-like operators at all and are
/// **not** implemented. What matters is that the report says so truthfully:
/// they are real amsmath commands, the message names the construction that
/// is missing, and the recovery is stated.
#[test]
fn the_two_arrow_operators_are_diagnosed_honestly() {
    for command in ["varinjlim", "varprojlim"] {
        let messages = diagnostics(&format!("\\{command}"));
        assert_eq!(messages.len(), 1, "\\{command}: {messages:?}");
        let message = &messages[0];
        assert!(message.contains("amsmath operator"), "{message}");
        assert!(message.contains("cleaders"), "{message}");
        assert!(
            !message.contains("is not supported in math mode"),
            "the old message read like a typo report: {message}"
        );
        // Both are known commands, so the editor must not offer them as
        // spelling corrections either.
        assert!(flashtex_compiler::vocabulary::is_known_command(command), "\\{command}");
    }
}

/// The four that are implemented are advertised, and the two that are not
/// are absent from the inventory rather than promised.
#[test]
fn the_inventory_says_exactly_what_is_implemented() {
    let inventory = flashtex_compiler::supported::inventory();
    let named = |name: &str| inventory.commands.iter().any(|c| c.name == name && c.renders);
    for command in ["injlim", "projlim", "varlimsup", "varliminf"] {
        assert!(named(command), "\\{command} is implemented but not advertised");
    }
    for command in ["varinjlim", "varprojlim"] {
        assert!(!named(command), "\\{command} is advertised but not implemented");
    }
}
