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

/// Every atom's (nucleus glyph or text, limits), for the `largesymbols`
/// operators and `\mathop{...}` below.
fn all_limits(formula: &str) -> Vec<(String, Option<Limits>)> {
    atoms("", formula)
        .into_iter()
        .map(|atom| {
            let name = match &atom.nucleus {
                Nucleus::Symbol(s) | Nucleus::Text(s) => s.clone(),
                Nucleus::Group(_) => "group".to_string(),
                other => format!("{other:?}"),
            };
            (name, atom.limits)
        })
        .collect()
}

/// TeX §1159: a limit switch sets the tail noad when it is an Op noad, so
/// it reaches `\bigcup`, `\bigcap` and the other `largesymbols` operators,
/// the integrals and a `\mathop{...}` exactly as it reaches `\lim`. Without
/// one they carry `None` (the renderer's default: `\displaylimits`, the
/// integrals `\nolimits`).
#[test]
fn a_switch_after_a_large_operator_sets_it() {
    let one = |s: &str, l: Option<Limits>| (s.to_string(), l);
    assert_eq!(all_limits("\\bigcup_{i=1}^n"), [one("⋃", None)]);
    assert_eq!(all_limits("\\bigcup\\limits_{i=1}^n"), [one("⋃", Some(Limits::Limits))]);
    assert_eq!(all_limits("\\bigcap\\nolimits_{i}"), [one("⋂", Some(Limits::NoLimits))]);
    assert_eq!(all_limits("\\sum\\displaylimits_{i}"), [one("∑", Some(Limits::DisplayLimits))]);
    assert_eq!(all_limits("\\int\\limits_0^1"), [one("∫", Some(Limits::Limits))]);
    // After the scripts too: the switch still sets the tail Op noad.
    assert_eq!(all_limits("\\bigcup_{i}\\limits^{n}"), [one("⋃", Some(Limits::Limits))]);
    // The last switch wins.
    assert_eq!(all_limits("\\bigoplus\\limits\\nolimits_i"), [one("⨁", Some(Limits::NoLimits))]);
    for (command, glyph) in [
        ("bigsqcup", "⨆"),
        ("bigvee", "⋁"),
        ("bigwedge", "⋀"),
        ("bigoplus", "⨁"),
        ("bigotimes", "⨂"),
        ("bigodot", "⨀"),
        ("biguplus", "⨄"),
        ("coprod", "∐"),
        ("prod", "∏"),
        ("oint", "∮"),
    ] {
        assert_eq!(all_limits(&format!("\\{command}\\limits_i")), [one(glyph, Some(Limits::Limits))], "\\{command}");
    }
    assert_eq!(all_limits("\\mathop{X}\\limits_k"), [one("group", Some(Limits::Limits))]);
}

/// A switch after anything that is not an Op noad is TeX's "Limit controls
/// must follow a math operator" and changes nothing.
#[test]
fn a_switch_after_a_non_operator_is_ignored() {
    assert_eq!(all_limits("x\\limits_i"), [("x".to_string(), None)]);
    assert_eq!(all_limits("\\cup\\limits_i"), [("∪".to_string(), None)]);
}
