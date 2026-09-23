//! A named operator's limit placement is on its atom (`MathAtom::limits`,
//! PLAN1 site 45): the kernel's declaration (`\lim` keeps `\mathop`'s
//! default `\displaylimits`, `\sin` is `\nolimits`), with a following
//! `\limits`/`\nolimits`/`\displaylimits` applied, whether the operator
//! was written directly or produced by a macro. `\mathrm{lim}` is an
//! ordinary run and carries none.
use flashtex_compiler::math::{Limits, MathAtom, Nucleus};
use flashtex_compiler::parser::{parse, Block, Inline};

fn atoms(preamble: &str, formula: &str) -> Vec<MathAtom> {
    let source = format!("\\documentclass{{article}}\n{preamble}\\begin{{document}}\n${formula}$\n\\end{{document}}\n");
    let parsed = parse(&source);
    parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => inlines.iter().find_map(|inline| match inline {
                Inline::Math { list, .. } => Some(list.atoms.clone()),
                _ => None,
            }),
            _ => None,
        })
        .expect("a formula")
}

fn limits_of(preamble: &str, formula: &str) -> Vec<(String, Option<Limits>)> {
    atoms(preamble, formula)
        .into_iter()
        .filter_map(|atom| match atom.nucleus {
            Nucleus::Text(text) => Some((text, atom.limits)),
            _ => None,
        })
        .collect()
}

#[test]
fn declared_placement() {
    assert_eq!(limits_of("", "\\lim_{n} a"), [("lim".to_string(), Some(Limits::DisplayLimits))]);
    assert_eq!(limits_of("", "\\sin x"), [("sin".to_string(), Some(Limits::NoLimits))]);
    assert_eq!(limits_of("", "\\mathrm{lim} x"), [("lim".to_string(), None)]);
}

#[test]
fn a_switch_after_the_operator_sets_it() {
    assert_eq!(limits_of("", "\\lim\\nolimits_{n} a"), [("lim".to_string(), Some(Limits::NoLimits))]);
    assert_eq!(limits_of("", "\\sin\\limits_{n} x"), [("sin".to_string(), Some(Limits::Limits))]);
    assert_eq!(limits_of("", "\\max \\nolimits\\displaylimits_{n} x"), [("max".to_string(), Some(Limits::DisplayLimits))]);
}

#[test]
fn an_operator_from_a_macro_carries_it_too() {
    assert_eq!(limits_of("\\newcommand\\lm{\\lim}\n", "\\lm_{n} a"), [("lim".to_string(), Some(Limits::DisplayLimits))]);
    assert_eq!(limits_of("\\newcommand\\lmn{\\lim\\limits}\n", "\\lmn_{n} a"), [("lim".to_string(), Some(Limits::Limits))]);
}
