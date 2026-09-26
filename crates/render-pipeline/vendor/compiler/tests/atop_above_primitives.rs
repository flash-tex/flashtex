//! TeX infix generalized fractions end to end under plain article (kernel
//! primitives need no package): no diagnostics, one stacked Inner fraction
//! per formula, the 1pt bar where one belongs.

use flashtex_compiler::math::{AtomClass, Nucleus};
use flashtex_compiler::parser::{parse, Block, Inline};

fn math_lists(document: &str) -> Vec<flashtex_compiler::math::MathList> {
    let parsed = parse(&format!(
        "\\documentclass{{article}}\n\\begin{{document}}\n{document}\n\\end{{document}}\n"
    ));
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let mut out = Vec::new();
    for block in &parsed.blocks {
        let Block::Paragraph(content) = block else { continue };
        for inline in content {
            if let Inline::Math { list, .. } = inline {
                out.push(list.clone());
            }
        }
    }
    out
}

fn single_fraction(
    lists: &[flashtex_compiler::math::MathList],
    index: usize,
) -> (Vec<String>, Vec<String>, Option<f64>, String, String) {
    let atoms = &lists[index].atoms;
    assert_eq!(atoms.len(), 1, "formula {index}: {atoms:?}");
    let atom = &atoms[0];
    assert_eq!(
        atom.class_override,
        Some(AtomClass::Inner),
        "formula {index}: an Inner atom like \\over"
    );
    match &atom.nucleus {
        Nucleus::GenFraction { numerator, denominator, thickness_pt, left, right, .. } => {
            let texts = |list: &flashtex_compiler::math::MathList| {
                list.atoms
                    .iter()
                    .map(|a| match &a.nucleus {
                        Nucleus::Symbol(s) => s.clone(),
                        other => panic!("formula {index}: expected a symbol, got {other:?}"),
                    })
                    .collect()
            };
            (texts(numerator), texts(denominator), *thickness_pt, left.clone(), right.clone())
        }
        other => panic!("formula {index}: expected a generalized fraction, got {other:?}"),
    }
}

#[test]
fn kernel_infix_fractions_match_pdflatex_without_errors() {
    // The lane's kernel case: pdflatex reports 0 errors and stacks all three.
    let lists = math_lists("$a\\atop b$ $x\\above 1pt y$ $p\\atopwithdelims() q$");
    assert_eq!(lists.len(), 3, "{lists:?}");
    assert_eq!(single_fraction(&lists, 0), (vec!["a".into()], vec!["b".into()], Some(0.0), String::new(), String::new()));
    assert_eq!(
        single_fraction(&lists, 1),
        (vec!["x".into()], vec!["y".into()], Some(1.0), String::new(), String::new())
    );
    assert_eq!(
        single_fraction(&lists, 2),
        (vec!["p".into()], vec!["q".into()], Some(0.0), "(".into(), ")".into())
    );
}

#[test]
fn delimited_default_and_explicit_rules_match_pdflatex() {
    let lists = math_lists("$a\\overwithdelims() b$ $c\\abovewithdelims[] 2pt d$");
    assert_eq!(lists.len(), 2, "{lists:?}");
    assert_eq!(
        single_fraction(&lists, 0),
        (vec!["a".into()], vec!["b".into()], None, "(".into(), ")".into())
    );
    assert_eq!(
        single_fraction(&lists, 1),
        (vec!["c".into()], vec!["d".into()], Some(2.0), "[".into(), "]".into())
    );
}
