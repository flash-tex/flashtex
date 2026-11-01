//! Plain-TeX `\root <index> \of <radicand>` builds the same radical
//! structure as LaTeX's `\sqrt[<index>]{<radicand>}`: the index is a general
//! math-mode token list terminated by `\of`, not a bracketed argument.

use flashtex_compiler::math::{MathList, Nucleus};
use flashtex_compiler::parser::{parse, Block, Inline};

fn math_lists(document: &str) -> Vec<MathList> {
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

/// The plain symbols of a flat list, so the comparison below never touches
/// source spans (which legitimately differ between the two formulas).
fn symbols(list: &MathList) -> Vec<String> {
    list.atoms
        .iter()
        .map(|atom| match &atom.nucleus {
            Nucleus::Symbol(s) => s.clone(),
            other => panic!("expected a symbol, got {other:?}"),
        })
        .collect()
}

/// Splits a rooted-radical formula into (index symbols, body symbols, index
/// carrier shift code): the carrier is the zero-width space holding the
/// index as its superscript, immediately ahead of the radical atom.
fn radical_parts(list: &MathList) -> (Vec<String>, Vec<String>, Option<f64>) {
    let atoms = &list.atoms;
    assert_eq!(atoms.len(), 2, "carrier plus radical: {atoms:?}");
    let carrier = &atoms[0];
    assert!(
        matches!(
            carrier.nucleus,
            Nucleus::Space { em, font_em: false } if em == 0.0
        ),
        "index carrier: {carrier:?}"
    );
    let index = carrier
        .superscript
        .as_ref()
        .expect("carrier holds the index");
    let Nucleus::Radical(body) = &atoms[1].nucleus else {
        panic!("radical atom: {:?}", atoms[1]);
    };
    (symbols(index), symbols(body), carrier.width_em)
}

#[test]
fn root_of_matches_sqrt_bracket_shape() {
    let lists = math_lists("$\\root 3\\of{x}$ $\\sqrt[3]{x}$");
    assert_eq!(lists.len(), 2, "{lists:?}");
    assert_eq!(radical_parts(&lists[0]), radical_parts(&lists[1]));
    assert_eq!(
        radical_parts(&lists[0]),
        (vec!["3".to_string()], vec!["x".to_string()], lists[0].atoms[0].width_em)
    );
}

#[test]
fn root_of_keeps_a_multi_token_index_whole() {
    let lists = math_lists("$\\root n+1 \\of {a+b}$");
    assert_eq!(lists.len(), 1, "{lists:?}");
    let (index, body, _) = radical_parts(&lists[0]);
    assert_eq!(index, vec!["n".to_string(), "+".to_string(), "1".to_string()]);
    assert_eq!(body, vec!["a".to_string(), "+".to_string(), "b".to_string()]);
}
